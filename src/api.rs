use std::sync::{Arc, Mutex};
use crate::types::{ChatMessage, ChatRequest, ChatResponse, ModelInfo, ModelsResponse};

pub async fn fetch_models(base_url: &str) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
    let url = format!("{}/api/tags", base_url);
    let response = reqwest::get(&url).await?;
    let models_response: ModelsResponse = response.json().await?;
    Ok(models_response.models)
}

pub async fn send_chat_request(
    base_url: &str,
    model: &str,
    conversation: &Arc<Mutex<Vec<ChatMessage>>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let messages = {
        let conversation = conversation.lock().unwrap();
        conversation.iter().cloned().collect::<Vec<_>>()
    };

    let request = ChatRequest {
        model: model.to_string(),
        messages,
        stream: false,
    };

    let client = reqwest::Client::new();
    let url = format!("{}/api/chat", base_url);
    
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await?;

    let chat_response: ChatResponse = response.json().await?;
    Ok(chat_response.message.content)
}
