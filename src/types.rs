use serde::{Deserialize, Serialize};

// --- Types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// Thinking content returned by Ollama when `think: true`. Only present in responses;
    /// skipped when serializing outgoing request messages.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub thinking: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    /// Generation temperature. Omitted from JSON when `None` so Ollama uses its default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Enable thinking mode (DeepSeek R1, Qwen 3). Omitted when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub think: Option<bool>,
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

    #[test]
    fn chat_message_serializes_and_deserializes() {
        // Verify that serde roundtrips correctly — if you ever rename a field
        // or change its type, this test catches it before it reaches the API.
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            thinking: None,
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
            temperature: None,
            think: None,
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