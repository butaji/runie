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
    #[serde(skip)]
    pub thought_duration_secs: Option<f64>,  // Duration of thinking (static)
}

impl AppState {
    /// Get elapsed thinking time in seconds
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.thinking_started_at.map(|start| start.elapsed().as_secs_f64())
    }
    
    /// Get thought duration in seconds (static value)
    pub fn thought_duration_secs(&self) -> Option<f64> {
        self.thought_duration_secs
    }
}

/// A chat message
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,      // "user", "assistant", "thought"
    pub content: String,
    pub timestamp: f64,    // Unix timestamp for ordering
}

/// Messages for agent communication
#[derive(Debug, Clone)]
pub enum Msg {
    User(String),
    Assistant(String),
}
