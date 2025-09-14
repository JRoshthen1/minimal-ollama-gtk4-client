use std::sync::{Arc, Mutex};
use futures_util::StreamExt;
use tokio::time::{timeout, Duration};
use crate::types::{ChatMessage, ChatRequest, ModelInfo, ModelsResponse, StreamResponse};

pub async fn fetch_models(base_url: &str) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/api/tags", base_url);
    
    // Add timeout to prevent hanging
    let response = timeout(Duration::from_secs(10), reqwest::get(&url)).await??;
    let models_response: ModelsResponse = response.json().await?;
    Ok(models_response.models)
}

pub async fn send_chat_request_streaming(
    base_url: &str,
    model: &str,
    conversation: &Arc<Mutex<Vec<ChatMessage>>>,
    token_sender: async_channel::Sender<String>,
) -> Result<(String, Option<String>), Box<dyn std::error::Error + Send + Sync>> {
    let messages = {
        let conversation = conversation.lock().unwrap();
        conversation.iter().cloned().collect::<Vec<_>>()
    };

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
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let mut current_batch = String::new();
    let mut tokens_since_last_send = 0;
    const BATCH_SIZE: usize = 20;
    const BATCH_TIMEOUT: Duration = Duration::from_millis(100);

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
                    let should_send = tokens_since_last_send >= BATCH_SIZE 
                        || last_send.elapsed() >= BATCH_TIMEOUT 
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
        return Err("No response received from the model".into());
    }

    Ok((full_response, None))
}