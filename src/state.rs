use std::sync::{Arc, Mutex};
use crate::types::ChatMessage;

#[derive(Clone)]
pub struct AppState {
    pub conversation: Arc<Mutex<Vec<ChatMessage>>>,
    pub ollama_url: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            conversation: Arc::new(Mutex::new(Vec::new())),
            ollama_url: "http://localhost:11434".to_string(),
        }
    }
}
