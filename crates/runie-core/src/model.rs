//! Model - Application State
use serde::{Deserialize, Serialize};

/// Main application state (immutable)
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub streaming: bool,
    pub scroll: usize,
    #[serde(skip)]
    pub thinking_started_at: Option<std::time::Instant>,
}

impl AppState {
    /// Get elapsed thinking time in seconds
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.thinking_started_at.map(|start| start.elapsed().as_secs_f64())
    }
}

/// A chat message
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,  // "user", "assistant", "thinking"
    pub content: String,
}

/// Messages for agent communication
#[derive(Debug, Clone)]
pub enum Msg {
    User(String),
    Assistant(String),
}
