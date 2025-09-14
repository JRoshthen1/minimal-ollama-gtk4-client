use std::rc::Rc;
use std::cell::RefCell;
use tokio::task::JoinHandle;
use crate::types::ChatMessage;
use crate::config::Config;

pub type SharedState = Rc<RefCell<AppState>>;

#[derive(Debug)]
pub enum AppError {
    Api(String),
    Ui(String),
    State(String),
    Validation(String),
    Config(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Api(msg) => write!(f, "API Error: {}", msg),
            AppError::Ui(msg) => write!(f, "UI Error: {}", msg),
            AppError::State(msg) => write!(f, "State Error: {}", msg),
            AppError::Validation(msg) => write!(f, "Validation Error: {}", msg),
            AppError::Config(msg) => write!(f, "Config Error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

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
    pub config: Config,
}

impl Default for AppState {
    fn default() -> Self {
        let config = Config::load().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config, using defaults: {}", e);
            Config::default()
        });
        
        Self {
            conversation: Vec::new(),
            ollama_url: config.ollama.url.clone(),
            is_generating: false,
            button_state: ButtonState::Send,
            current_task: None,
            selected_model: None,
            status_message: "Ready".to_string(),
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