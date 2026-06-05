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
    pub thought_duration_secs: Option<f64>,
    pub request_queue: Vec<(String, String)>,
    #[serde(skip)]
    pub next_id: u64,
    #[serde(skip)]
    pub current_request_id: Option<String>,
    #[serde(skip)]
    pub turn_started_at: Option<std::time::Instant>,
}

impl AppState {
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.thinking_started_at.map(|start| start.elapsed().as_secs_f64())
    }
    
    pub fn thought_duration_secs(&self) -> Option<f64> {
        self.thought_duration_secs
    }
    
    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.turn_started_at.map(|start| start.elapsed().as_secs_f64())
    }
    
    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.next_id);
        self.next_id += 1;
        id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: f64,
    pub id: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    User(String),
    Assistant(String),
}
