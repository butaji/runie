//! Message formatting logic
//! 
//! Converts ChatMessage + state metadata into display lines.

use crate::model::{AppState, ChatMessage};
use crate::labels::{PREFIX_USER, PREFIX_AGENT};

/// A formatted line ready for rendering
#[derive(Debug, Clone)]
pub struct DisplayLine {
    pub spans: Vec<DisplaySpan>,
}

/// A single styled span of text
#[derive(Debug, Clone)]
pub struct DisplaySpan {
    pub text: String,
    pub color: Option<Color>,
}

/// Terminal colors
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Cyan,
    Green,
    Yellow,
    DarkGray,
    White,
}

/// Format all messages for display
pub fn format_messages(state: &AppState) -> Vec<DisplayLine> {
    let mut lines = vec![];
    let mut last_was_assistant = false;
    
    for msg in &state.messages {
        match msg.role.as_str() {
            "user" => {
                lines.extend(user_message(&msg.content));
                last_was_assistant = false;
            }
            "thought" => {
                lines.extend(thought_message(&msg.content));
                last_was_assistant = false;
            }
            "assistant" => {
                if last_was_assistant {
                    if let Some(last) = lines.last_mut() {
                        if last.spans.len() > 1 {
                            last.spans[1].text.push_str(&msg.content);
                        }
                    }
                } else {
                    lines.extend(agent_answer(&msg.content));
                }
                last_was_assistant = true;
            }
            _ => {}
        }
    }
    
    // Show thinking indicator only if no thought message yet (still streaming)
    let has_thought = state.messages.iter().any(|m| m.role == "thought");
    if !has_thought && (state.streaming || !state.request_queue.is_empty()) {
        lines.extend(thinking_indicator(state));
    }
    
    lines
}

// === Message Component Formatters ===

/// User message
pub fn user_message(content: &str) -> Vec<DisplayLine> {
    vec![
        DisplayLine {
            spans: vec![
                DisplaySpan { text: PREFIX_USER.to_string(), color: Some(Color::Cyan) },
                DisplaySpan { text: content.to_string(), color: None },
            ],
        },
        DisplayLine::empty(),
    ]
}

/// Agent answer
pub fn agent_answer(content: &str) -> Vec<DisplayLine> {
    vec![
        DisplayLine {
            spans: vec![
                DisplaySpan { text: PREFIX_AGENT.to_string(), color: Some(Color::Green) },
                DisplaySpan { text: content.to_string(), color: None },
            ],
        },
        DisplayLine::empty(),
    ]
}

/// Bailer spinner characters
const SPINNER_FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

/// Thinking indicator (animated): "⠋ Thinking... Xs"
pub fn thinking_indicator(state: &AppState) -> Vec<DisplayLine> {
    let elapsed = state.thinking_elapsed_secs().unwrap_or(0.0);
    let frame_idx = ((elapsed * 10.0) as usize) % SPINNER_FRAMES.len();
    let spinner = SPINNER_FRAMES[frame_idx];
    let text = format!("{} Thinking... {:.1}s", spinner, elapsed);
    
    vec![
        DisplayLine {
            spans: vec![DisplaySpan { text, color: Some(Color::DarkGray) }],
        },
        DisplayLine::empty(),
    ]
}

/// Thought message (stored): "◆ Thought Xs"
pub fn thought_message(content: &str) -> Vec<DisplayLine> {
    vec![
        DisplayLine {
            spans: vec![DisplaySpan { text: content.to_string(), color: Some(Color::DarkGray) }],
        },
        DisplayLine::empty(),
    ]
}

impl DisplayLine {
    pub fn empty() -> Self {
        DisplayLine { spans: vec![] }
    }
    
    pub fn is_empty(&self) -> bool {
        self.spans.iter().all(|s| s.text.is_empty())
    }
}
