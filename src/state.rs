use std::rc::Rc;
use std::cell::RefCell;
use tokio::task::JoinHandle;
use crate::types::ChatMessage;
use crate::config::Config;

pub type SharedState = Rc<RefCell<AppState>>;

/// Application-level errors. Uses `thiserror` so each variant gets a clear, typed
/// message without boilerplate. Callers can match on the variant to handle errors
/// differently (e.g. show a dialog for Config vs. a status-bar message for Api).
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("API error: {0}")]
    Api(String),
    #[error("UI error: {0}")]
    Ui(String),
    #[error("State error: {0}")]
    State(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Config error: {0}")]
    Config(String),
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonState {
    Send,
    Stop,
}

pub struct AppState {
    pub conversation: Vec<ChatMessage>,
    pub ollama_url: String,
    pub is_generating: bool,
    pub button_state: ButtonState,
    pub current_task: Option<JoinHandle<()>>,
    pub selected_model: Option<String>,
    pub status_message: String,
    /// System prompt prepended to every request. Initialized from config but can be
    /// overridden at runtime (e.g. by a RAG pipeline to inject retrieved context).
    pub system_prompt: Option<String>,
    pub config: Config,
}

impl Default for AppState {
    fn default() -> Self {
        let config = Config::load().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config, using defaults: {}", e);
            Config::default()
        });

        let system_prompt = if config.ollama.system_prompt.is_empty() {
            None
        } else {
            Some(config.ollama.system_prompt.clone())
        };

        Self {
            conversation: Vec::new(),
            ollama_url: config.ollama.url.clone(),
            is_generating: false,
            button_state: ButtonState::Send,
            current_task: None,
            selected_model: None,
            status_message: "Ready".to_string(),
            system_prompt,
            config,
        }
    }
}

impl AppState {
    pub fn set_generating(&mut self, generating: bool) {
        self.is_generating = generating;
        self.button_state = if generating {
            ButtonState::Stop
        } else {
            ButtonState::Send
        };
    }

    pub fn add_user_message(&mut self, content: String) {
        self.conversation.push(ChatMessage {
            role: "user".to_string(),
            content,
        });
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.conversation.push(ChatMessage {
            role: "assistant".to_string(),
            content
        });
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = message;
    }

    pub fn abort_current_task(&mut self) {
        if let Some(task) = self.current_task.take() {
            task.abort();
        }
        self.set_generating(false);
        self.set_status("Generation stopped".to_string());
    }
    
}