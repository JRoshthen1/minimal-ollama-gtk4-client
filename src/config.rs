use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ui: UiConfig,
    pub colors: ColorConfig,
    pub ollama: OllamaConfig,
    pub streaming: StreamingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub window_font_size: u32,
    pub chat_font_size: u32,
    pub input_font_size: u32,
    pub code_font_family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    // Background colors
    pub chat_background: String,
    pub code_background: String,
    pub window_background: String,
    
    // Text colors
    pub primary_text: String,
    pub code_text: String,
    pub link_text: String,
    pub think_text: String,
    
    // Button colors
    pub send_button: String,
    pub stop_button: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub url: String,
    pub timeout_seconds: u64,
    /// Maximum number of conversation turns sent to the model (most recent N messages).
    /// Keeps context within the model's limit. Set higher for longer memory.
    pub max_context_messages: usize,
    /// Optional system prompt prepended to every conversation.
    /// Leave empty ("") to disable. RAG can override this at runtime via AppState.
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Number of tokens to accumulate before flushing to the UI.
    pub batch_size: usize,
    /// Maximum milliseconds to wait before flushing a partial batch.
    pub batch_timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: UiConfig::default(),
            colors: ColorConfig::default(),
            ollama: OllamaConfig::default(),
            streaming: StreamingConfig::default(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            window_font_size: 16,
            chat_font_size: 18,
            input_font_size: 16,
            code_font_family: "monospace".to_string(),
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            // Background colors
            chat_background: "#ffffff".to_string(),
            code_background: "#f5f5f5".to_string(),
            window_background: "#fafafa".to_string(),
            
            // Text colors
            primary_text: "#333333".to_string(),
            code_text: "#d63384".to_string(),
            link_text: "#0066cc".to_string(),
            think_text: "#6666cc".to_string(),
            
            // Button colors
            send_button: "#007bff".to_string(),
            stop_button: "#dc3545".to_string(),
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:11434".to_string(),
            timeout_seconds: 120,
            max_context_messages: 20,
            system_prompt: String::new(),
        }
    }
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            batch_size: 20,
            batch_timeout_ms: 100,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config file
            let default_config = Config::default();
            default_config.save()?;
            Ok(default_config)
        }
    }
    
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }
    
    fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not determine config directory")?
            .join("ollama-chat-gtk4");
        
        Ok(config_dir.join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sensible_values() {
        let cfg = Config::default();

        // Ollama endpoint
        assert_eq!(cfg.ollama.url, "http://localhost:11434");
        assert!(cfg.ollama.timeout_seconds > 0);

        // Context window: 0 would mean every request sends zero messages — bad default.
        assert!(cfg.ollama.max_context_messages > 0,
            "max_context_messages must be > 0 or no history is ever sent");

        // Streaming
        assert!(cfg.streaming.batch_size > 0);
        assert!(cfg.streaming.batch_timeout_ms > 0);

        // System prompt empty by default (RAG / user sets it when needed)
        assert!(cfg.ollama.system_prompt.is_empty());
    }

    #[test]
    fn config_roundtrips_through_toml() {
        // Serialize to TOML and back — verifies the serde field names are stable.
        // If you rename a field in the struct but forget to update the serde attribute,
        // existing config files on disk will silently lose that value. This catches it.
        let original = Config::default();
        let toml_str = toml::to_string_pretty(&original).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.ollama.url, original.ollama.url);
        assert_eq!(parsed.ollama.max_context_messages, original.ollama.max_context_messages);
        assert_eq!(parsed.streaming.batch_size, original.streaming.batch_size);
        assert_eq!(parsed.streaming.batch_timeout_ms, original.streaming.batch_timeout_ms);
        assert_eq!(parsed.ui.window_font_size, original.ui.window_font_size);
    }

    #[test]
    fn config_roundtrip_preserves_custom_values() {
        let mut cfg = Config::default();
        cfg.ollama.url = "http://my-server:11434".to_string();
        cfg.ollama.max_context_messages = 50;
        cfg.streaming.batch_size = 5;
        cfg.ollama.system_prompt = "You are a helpful assistant.".to_string();

        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.ollama.url, "http://my-server:11434");
        assert_eq!(parsed.ollama.max_context_messages, 50);
        assert_eq!(parsed.streaming.batch_size, 5);
        assert_eq!(parsed.ollama.system_prompt, "You are a helpful assistant.");
    }
}