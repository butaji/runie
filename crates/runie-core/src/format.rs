//! Message formatting logic
//! 
//! Converts ChatMessage + state metadata into display lines.

use crate::model::{AppState, ChatMessage};

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

/// Format a single message into display lines
pub fn format_message(msg: &ChatMessage) -> Vec<DisplayLine> {
    match msg.role.as_str() {
        "user" => format_user_message(msg),
        "assistant" => format_assistant_message(msg),
        _ => vec![],
    }
}

/// Format all messages for display, including thinking indicator
pub fn format_messages(state: &AppState) -> Vec<DisplayLine> {
    let mut lines = vec![];
    let mut saw_user = false;
    let mut saw_assistant = false;
    
    for msg in &state.messages {
        lines.extend(format_message(msg));
        
        if msg.role == "user" {
            saw_user = true;
        }
        if msg.role == "assistant" {
            saw_assistant = true;
        }
        
        // Show thinking indicator between user and assistant
        if saw_user && !saw_assistant && state.streaming {
            // Still thinking - show live timer
            let elapsed = state.thinking_elapsed_secs()
                .map(|s| format!("⏳ Thinking... {:.1}s", s))
                .unwrap_or_else(|| "⏳ Thinking...".to_string());
            
            lines.push(DisplayLine {
                spans: vec![DisplaySpan {
                    text: elapsed,
                    color: Some(Color::DarkGray),
                }],
            });
            lines.push(DisplayLine::empty());
        }
        
        // Show thought time after assistant response starts
        if msg.role == "assistant" && !msg.content.is_empty() {
            if let Some(duration) = state.thought_duration_secs() {
                lines.push(DisplayLine {
                    spans: vec![DisplaySpan {
                        text: format!("⏳ Thought {:.1}s", duration),
                        color: Some(Color::DarkGray),
                    }],
                });
                lines.push(DisplayLine::empty());
            }
        }
    }
    
    lines
}

fn format_user_message(msg: &ChatMessage) -> Vec<DisplayLine> {
    vec![
        DisplayLine {
            spans: vec![
                DisplaySpan { text: "You: ".to_string(), color: Some(Color::Cyan) },
                DisplaySpan { text: msg.content.clone(), color: None },
            ],
        },
        DisplayLine::empty(),
    ]
}

fn format_assistant_message(msg: &ChatMessage) -> Vec<DisplayLine> {
    vec![
        DisplayLine {
            spans: vec![
                DisplaySpan { text: "Agent: ".to_string(), color: Some(Color::Green) },
                DisplaySpan { text: msg.content.clone(), color: None },
            ],
        },
        DisplayLine::empty(),
    ]
}

impl DisplayLine {
    pub fn empty() -> Self {
        DisplayLine { spans: vec![] }
    }
}
