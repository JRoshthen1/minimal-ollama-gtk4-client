use serde::{Deserialize, Serialize};

// --- Types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub message: ChatMessage,
    pub done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamResponse {
    pub model: String,
    pub created_at: String,
    pub message: ChatMessage,
    pub done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // `cargo test` will run these. This is the standard Rust pattern:
    // put tests in a `#[cfg(test)]` module so they're compiled only when testing.

    #[test]
    fn chat_message_serializes_and_deserializes() {
        // Verify that serde roundtrips correctly — if you ever rename a field
        // or change its type, this test catches it before it reaches the API.
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.role, msg.role);
        assert_eq!(decoded.content, msg.content);
    }

    #[test]
    fn chat_request_includes_stream_flag() {
        let req = ChatRequest {
            model: "llama3".to_string(),
            messages: vec![],
            stream: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        // The Ollama API requires `"stream": true` in the body.
        assert!(json.contains("\"stream\":true"));
    }

    #[test]
    fn stream_response_parses_done_flag() {
        // Real payload shape from the Ollama streaming API.
        let raw = r#"{"model":"llama3","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hi"},"done":true}"#;
        let resp: StreamResponse = serde_json::from_str(raw).unwrap();
        assert!(resp.done);
        assert_eq!(resp.message.content, "Hi");
    }
}