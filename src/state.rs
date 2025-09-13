use std::sync::{Arc, Mutex};
use crate::types::ChatMessage;

#[derive(Clone)]
pub struct AppState {
    pub conversation: Arc<Mutex<Vec<ChatMessage>>>,
    pub ollama_url: String,
    pub thinking_enabled: Arc<Mutex<bool>>,
    pub current_request_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            conversation: Arc::new(Mutex::new(Vec::new())),
            ollama_url: "http://localhost:11434".to_string(),
            thinking_enabled: Arc::new(Mutex::new(false)),
            current_request_handle: Arc::new(Mutex::new(None)),
        }
    }
}