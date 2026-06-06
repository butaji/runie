//! Model - Application State
use serde::{Deserialize, Serialize};

const MAX_MESSAGES: usize = 10_000;

#[derive(Serialize, Deserialize, Clone)]
pub struct AppState {
    pub messages: Vec<ChatMessage>,
    #[serde(skip)]
    pub message_start: usize,  // Ring buffer start index
    pub input: String,
    pub streaming: bool,
    pub scroll: usize,
    #[serde(skip)]
    pub thinking_started_at: Option<std::time::Instant>,
    pub request_queue: Vec<(String, String)>,
    #[serde(skip)]
    pub next_id: u64,
    #[serde(skip)]
    pub current_request_id: Option<String>,
    #[serde(skip)]
    pub turn_started_at: Option<std::time::Instant>,
    #[serde(skip)]
    pub current_tool_name: Option<String>,
    #[serde(skip)]
    pub tool_started_at: Option<std::time::Instant>,
    #[serde(skip)]
    pub has_intermediate_steps: bool,  // True if tool or other steps occurred in this turn
    #[serde(skip)]
    pub animation_frame: u32,  // For animating spinners (0-11 cycles through braille chars)
    #[serde(skip)]
    pub turn_active: bool,  // True when a turn is in progress
    #[serde(skip)]
    pub current_action: Option<String>,  // Current action: "Thinking", "Running <tool>", etc.
    #[serde(skip)]
    pub formatted_cache: Vec<crate::ui::DisplayLine>,  // Cached formatted messages
    #[serde(skip)]
    pub element_count: usize,  // Cached element count for virtual list
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::with_capacity(MAX_MESSAGES),
            message_start: 0,
            input: String::new(),
            streaming: false,
            scroll: 0,
            thinking_started_at: None,
            request_queue: Vec::new(),
            next_id: 0,
            current_request_id: None,
            turn_started_at: None,
            current_tool_name: None,
            tool_started_at: None,
            has_intermediate_steps: false,
            animation_frame: 0,
            turn_active: false,
            current_action: None,
            formatted_cache: Vec::new(),
            element_count: 0,
        }
    }
}

impl AppState {
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.thinking_started_at.map(|start| start.elapsed().as_secs_f64())
    }
    
    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.turn_started_at.map(|start| start.elapsed().as_secs_f64())
    }
    
    pub fn tool_elapsed_secs(&self) -> Option<f64> {
        self.tool_started_at.map(|start| start.elapsed().as_secs_f64())
    }
    
    pub fn spinner_frame(&self) -> char {
        // Braille spinner: ⠋⠙⠹⠸⠼⠴⠦⠧⠹⠸⠴⠼ (12 frames)
        const SPINNERS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
        SPINNERS[(self.animation_frame % 12) as usize]
    }
    
    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.next_id);
        self.next_id += 1;
        id
    }
    
    /// Update cached element count - O(n) but only when messages change
    pub fn update_element_count(&mut self) {
        let mut count = 0;
        for msg in &self.messages {
            match msg.role.as_str() {
                "user" | "thought" | "assistant" | "tool" | "turn_complete" => {
                    count += 2; // element + spacer
                }
                _ => {}
            }
        }
        if self.thinking_started_at.is_some() {
            count += 2;
        }
        self.element_count = count;
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
