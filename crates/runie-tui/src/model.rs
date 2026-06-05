//! Model - Application State
use serde::{Deserialize, Serialize};

/// Main application state (immutable)
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub streaming: bool,
    pub scroll: usize,
}

/// A chat message
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Messages for agent communication
#[derive(Debug, Clone)]
pub enum Msg {
    User(String),
    Assistant(String),
}
