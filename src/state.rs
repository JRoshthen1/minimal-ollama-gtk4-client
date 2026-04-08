use std::rc::Rc;
use std::cell::RefCell;
use tokio::task::JoinHandle;
use crate::types::ChatMessage;
use crate::config::{Config, Profile};
use crate::db::Database;

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
    /// Currently active profile. `None` means use global config defaults.
    pub active_profile: Option<Profile>,
    /// SQLite database for profiles and future history/RAG. `None` if DB failed to open.
    pub db: Option<Database>,
    /// Whether to send `think: true` in API requests (DeepSeek R1, Qwen 3).
    pub thinking_enabled: bool,
    /// The currently open conversation's DB id. `None` means no conversation is active yet
    /// (a new one will be created on the first sent message).
    pub current_conversation_id: Option<i64>,
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

        let db: Option<Database> = match Database::open() {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("Warning: Failed to open database, profiles unavailable: {}", e);
                None
            }
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
            thinking_enabled: config.ollama.thinking_enabled,
            config,
            active_profile: None,
            db,
            current_conversation_id: None,
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
            thinking: None,
        });
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.conversation.push(ChatMessage {
            role: "assistant".to_string(),
            content,
            thinking: None,
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

    /// Clear the in-memory conversation and discard the active conversation id,
    /// making the next sent message start a brand-new conversation in the DB.
    pub fn clear_conversation(&mut self) {
        self.conversation.clear();
        self.current_conversation_id = None;
    }

    /// Apply a profile, updating system_prompt and storing the active profile.
    /// Pass `None` to clear the active profile and fall back to global config defaults.
    pub fn apply_profile(&mut self, profile: Option<Profile>) {
        self.system_prompt = match &profile {
            Some(p) if !p.system_prompt.is_empty() => Some(p.system_prompt.clone()),
            Some(_) => None,
            None => {
                if self.config.ollama.system_prompt.is_empty() {
                    None
                } else {
                    Some(self.config.ollama.system_prompt.clone())
                }
            }
        };
        self.active_profile = profile;
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> AppState {
        AppState {
            conversation: Vec::new(),
            ollama_url: "http://localhost:11434".into(),
            is_generating: false,
            button_state: ButtonState::Send,
            current_task: None,
            selected_model: None,
            status_message: "Ready".into(),
            system_prompt: None,
            config: Config::default(),
            active_profile: None,
            db: None,
            thinking_enabled: false,
            current_conversation_id: None,
        }
    }

    #[test]
    fn set_generating_true_sets_stop_state() {
        let mut state = make_state();
        state.set_generating(true);
        assert!(state.is_generating);
        assert_eq!(state.button_state, ButtonState::Stop);
    }

    #[test]
    fn set_generating_false_sets_send_state() {
        let mut state = make_state();
        state.is_generating = true;
        state.button_state = ButtonState::Stop;
        state.set_generating(false);
        assert!(!state.is_generating);
        assert_eq!(state.button_state, ButtonState::Send);
    }

    #[test]
    fn add_user_message_appends_with_correct_role() {
        let mut state = make_state();
        state.add_user_message("hello".into());
        assert_eq!(state.conversation.len(), 1);
        assert_eq!(state.conversation[0].role, "user");
        assert_eq!(state.conversation[0].content, "hello");
    }

    #[test]
    fn add_assistant_message_appends_with_correct_role() {
        let mut state = make_state();
        state.add_assistant_message("hi there".into());
        assert_eq!(state.conversation.len(), 1);
        assert_eq!(state.conversation[0].role, "assistant");
        assert_eq!(state.conversation[0].content, "hi there");
    }

    #[test]
    fn conversation_preserves_insertion_order() {
        let mut state = make_state();
        state.add_user_message("first".into());
        state.add_assistant_message("second".into());
        state.add_user_message("third".into());
        assert_eq!(state.conversation.len(), 3);
        assert_eq!(state.conversation[0].role, "user");
        assert_eq!(state.conversation[1].role, "assistant");
        assert_eq!(state.conversation[2].role, "user");
    }

    #[test]
    fn set_status_updates_message() {
        let mut state = make_state();
        state.set_status("Loading models...".into());
        assert_eq!(state.status_message, "Loading models...");
    }

    #[test]
    fn abort_current_task_without_task_resets_state() {
        let mut state = make_state();
        state.is_generating = true;
        state.button_state = ButtonState::Stop;
        state.abort_current_task();
        assert!(!state.is_generating);
        assert_eq!(state.button_state, ButtonState::Send);
        assert_eq!(state.status_message, "Generation stopped");
        assert!(state.current_task.is_none());
    }

    #[tokio::test]
    async fn abort_current_task_aborts_running_task() {
        let mut state = make_state();
        // Spawn a task that sleeps forever so we can verify it gets aborted
        let handle = tokio::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        });
        state.current_task = Some(handle);
        state.is_generating = true;
        state.button_state = ButtonState::Stop;

        state.abort_current_task();

        assert!(state.current_task.is_none());
        assert!(!state.is_generating);
        assert_eq!(state.status_message, "Generation stopped");
    }
}