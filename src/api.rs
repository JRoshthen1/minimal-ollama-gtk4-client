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
) -> Result<String, ApiError> {

    let request = ChatRequest {
        model: model.to_string(),
        messages,
        stream: true,
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