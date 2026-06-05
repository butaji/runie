//! Message formatting logic
//! 
//! Converts ChatMessage + state metadata into display lines.

use crate::model::AppState;
use crate::labels::{PREFIX_USER, PREFIX_AGENT, THINKING_LOADING, thinking_with_time};

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
                // Combine consecutive assistant messages
                if last_was_assistant {
                    // Append to last agent line's content span (index 1)
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
    
    // Show thinking indicator if streaming and no response yet
    if state.streaming && !last_was_assistant {
        lines.extend(thinking(state));
    }
    
    lines
}

// === Message Component Formatters ===

/// User message: "You: <content>"
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

/// Agent answer: "Agent: <content>"
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

/// Thinking indicator (live timer): "⏳ Thinking... X.Xs"
pub fn thinking(state: &AppState) -> Vec<DisplayLine> {
    let elapsed = state.thinking_elapsed_secs()
        .map(thinking_with_time)
        .unwrap_or_else(|| THINKING_LOADING.to_string());
    
    vec![
        DisplayLine {
            spans: vec![DisplaySpan { text: elapsed, color: Some(Color::DarkGray) }],
        },
        DisplayLine::empty(),
    ]
}

/// Thought message (stored): "⏳ Thought X.Xs"
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
