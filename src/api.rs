use futures_util::StreamExt;
use tokio::time::{timeout, Duration};
use crate::types::{ChatMessage, ChatRequest, ModelInfo, ModelsResponse, StreamResponse};

/// Typed errors for the Ollama API layer. Using `thiserror` means callers can match
/// on exactly what went wrong instead of downcasting a `Box<dyn Error>`.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Request timed out")]
    Timeout,
    #[error("Server returned error status {0}")]
    BadStatus(u16),
    #[error("Failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Model returned empty response")]
    EmptyResponse,
}

pub async fn fetch_models(base_url: &str) -> Result<Vec<ModelInfo>, ApiError> {
    let url = format!("{}/api/tags", base_url);

    let response = timeout(Duration::from_secs(10), reqwest::get(&url))
        .await
        .map_err(|_| ApiError::Timeout)??;
    let models_response: ModelsResponse = response.json().await?;
    Ok(models_response.models)
}

pub async fn send_chat_request_streaming(
    base_url: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    token_sender: async_channel::Sender<String>,
    batch_size: usize,
    batch_timeout_ms: u64,
    temperature: Option<f32>,
) -> Result<String, ApiError> {

    let request = ChatRequest {
        model: model.to_string(),
        messages,
        stream: true,
        temperature,
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120)) // 2 minute timeout
        .build()?;
    
    let url = format!("{}/api/chat", base_url);
    
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(ApiError::BadStatus(response.status().as_u16()));
    }

    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let mut current_batch = String::new();
    let mut tokens_since_last_send = 0;
    let batch_timeout = Duration::from_millis(batch_timeout_ms);

    let mut last_send = tokio::time::Instant::now();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        let text = String::from_utf8_lossy(&chunk);
        
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            match serde_json::from_str::<StreamResponse>(line) {
                Ok(stream_response) => {
                    let token = stream_response.message.content;
                    
                    if !token.is_empty() {
                        full_response.push_str(&token);
                        current_batch.push_str(&token);
                        tokens_since_last_send += 1;
                    }
                    
                    // Send batch if conditions are met
                    let should_send = tokens_since_last_send >= batch_size
                        || last_send.elapsed() >= batch_timeout
                        || stream_response.done;
                    
                    if should_send {
                        // Send content batch
                        if !current_batch.is_empty() {
                            match token_sender.send(current_batch.clone()).await {
                                Ok(_) => {
                                    current_batch.clear();
                                    tokens_since_last_send = 0;
                                }
                                Err(_) => break,
                            }
                        }

                        last_send = tokio::time::Instant::now();
                    }
                    
                    if stream_response.done {
                        break;
                    }
                }
                Err(parse_error) => {
                    // Log parse errors but continue processing
                    eprintln!("Failed to parse streaming response: {} (line: {})", parse_error, line);
                    continue;
                }
            }
        }
    }
    
    // Send any remaining tokens in the batches
    if !current_batch.is_empty() {
        let _ = token_sender.send(current_batch).await;
    }

    // Close channels
    drop(token_sender);

    if full_response.is_empty() {
        return Err(ApiError::EmptyResponse);
    }

    Ok(full_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── fetch_models ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn fetch_models_returns_model_list() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/api/tags")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"models":[{"name":"llama3","modified_at":"2024-01-01T00:00:00Z","size":4000000}]}"#)
            .create_async()
            .await;

        let models = fetch_models(&server.url()).await.unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "llama3");
    }

    #[tokio::test]
    async fn fetch_models_bad_status_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/api/tags")
            .with_status(500)
            .create_async()
            .await;

        let err = fetch_models(&server.url()).await.unwrap_err();
        // reqwest treats non-success as an error only if we explicitly check;
        // here fetch_models passes the status through response.json() which will
        // fail because body is empty — so we get an Http or Parse error.
        // The important thing: it is an error, not a success.
        assert!(matches!(err, ApiError::Http(_) | ApiError::Parse(_) | ApiError::BadStatus(_)));
    }

    #[tokio::test]
    async fn fetch_models_bad_json_returns_parse_error() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/api/tags")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not json")
            .create_async()
            .await;

        let err = fetch_models(&server.url()).await.unwrap_err();
        assert!(matches!(err, ApiError::Http(_) | ApiError::Parse(_)));
    }

    // ── send_chat_request_streaming ──────────────────────────────────────────

    fn ndjson_lines(tokens: &[(&str, bool)]) -> String {
        tokens
            .iter()
            .map(|(content, done)| {
                format!(
                    r#"{{"model":"llama3","created_at":"2024-01-01T00:00:00Z","message":{{"role":"assistant","content":"{}"}},"done":{}}}"#,
                    content, done
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    async fn run_streaming(server_url: &str, batch_size: usize) -> (Result<String, ApiError>, Vec<String>) {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "hi".to_string(),
        }];
        let (tx, rx) = async_channel::unbounded();
        let result = send_chat_request_streaming(
            server_url, "llama3", messages, tx, batch_size, 5000, None,
        )
        .await;

        let mut batches = Vec::new();
        while let Ok(batch) = rx.try_recv() {
            batches.push(batch);
        }
        (result, batches)
    }

    #[tokio::test]
    async fn streaming_single_token_returns_full_response() {
        let mut server = mockito::Server::new_async().await;
        let body = ndjson_lines(&[("Hello", true)]);
        let _mock = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;

        let (result, _batches) = run_streaming(&server.url(), 100).await;
        assert_eq!(result.unwrap(), "Hello");
    }

    #[tokio::test]
    async fn streaming_multi_token_accumulates_full_response() {
        let mut server = mockito::Server::new_async().await;
        let body = ndjson_lines(&[("Hello", false), (" world", true)]);
        let _mock = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;

        let (result, _) = run_streaming(&server.url(), 100).await;
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[tokio::test]
    async fn streaming_batch_size_flushes_intermediate_batches() {
        let mut server = mockito::Server::new_async().await;
        // 3 tokens, batch_size=2 → first batch sent after 2 tokens, second after done
        let body = ndjson_lines(&[("a", false), ("b", false), ("c", true)]);
        let _mock = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;

        let (result, batches) = run_streaming(&server.url(), 2).await;
        assert_eq!(result.unwrap(), "abc");
        // We should have received at least 2 channel messages (one mid-stream, one final)
        assert!(batches.len() >= 2, "expected intermediate batches, got {:?}", batches);
        assert_eq!(batches.join(""), "abc");
    }

    #[tokio::test]
    async fn streaming_done_with_no_content_returns_empty_response_error() {
        let mut server = mockito::Server::new_async().await;
        // done:true but content is empty
        let body = r#"{"model":"llama3","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":""},"done":true}"#;
        let _mock = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;

        let (result, _) = run_streaming(&server.url(), 100).await;
        assert!(matches!(result, Err(ApiError::EmptyResponse)));
    }

    #[tokio::test]
    async fn streaming_bad_status_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/api/chat")
            .with_status(503)
            .create_async()
            .await;

        let (result, _) = run_streaming(&server.url(), 100).await;
        assert!(matches!(result, Err(ApiError::BadStatus(503))));
    }

    #[tokio::test]
    async fn streaming_malformed_json_line_is_skipped() {
        let mut server = mockito::Server::new_async().await;
        // A bad line in the middle should not abort processing
        let body = format!(
            "{}\nnot valid json\n{}",
            r#"{"model":"llama3","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}"#,
            r#"{"model":"llama3","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":" world"},"done":true}"#,
        );
        let _mock = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_body(body)
            .create_async()
            .await;

        let (result, _) = run_streaming(&server.url(), 100).await;
        // Should still accumulate valid tokens despite the bad line
        assert_eq!(result.unwrap(), "Hello world");
    }
}