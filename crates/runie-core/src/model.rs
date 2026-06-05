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
    pub request_queue: Vec<(String, String)>,  // (content, id) pairs
    #[serde(skip)]
    pub next_id: u64,  // Next ID to assign
    #[serde(skip)]
    pub current_request_id: Option<String>,  // ID of currently processing request
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
    
    /// Generate and return next ID
    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.next_id);
        self.next_id += 1;
        id
    }
}

/// A chat message
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,      // "user", "assistant", "thought"
    pub content: String,
    pub timestamp: f64,    // Unix timestamp for ordering
    pub id: String,         // Composite ID with dot separator, e.g. "user.1", "thought.1", "agent.1"
}

/// Messages for agent communication
#[derive(Debug, Clone)]
pub enum Msg {
    User(String),
    Assistant(String),
}
