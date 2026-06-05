//! Message formatting logic
//! 
//! Converts ChatMessage + state metadata into display lines.

use crate::model::AppState;

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

/// Format all messages for display, including thinking indicator
pub fn format_messages(state: &AppState) -> Vec<DisplayLine> {
    let mut lines = vec![];
    
    for msg in &state.messages {
        match msg.role.as_str() {
            "user" => lines.extend(user_message(&msg.content)),
            "assistant" => {
                // Add thought indicator before first assistant response
                if lines.iter().all(|l| !l.is_thought()) && !msg.content.is_empty() {
                    lines.extend(thought_duration(state));
                }
                lines.extend(agent_answer(&msg.content));
            }
            _ => {}
        }
    }
    
    // Show thinking indicator if streaming and no response yet
    if state.streaming && !lines.iter().any(|l| l.is_agent()) {
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
                DisplaySpan { text: "You: ".to_string(), color: Some(Color::Cyan) },
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
                DisplaySpan { text: "Agent: ".to_string(), color: Some(Color::Green) },
                DisplaySpan { text: content.to_string(), color: None },
            ],
        },
        DisplayLine::empty(),
    ]
}

/// Thinking indicator (live timer): "⏳ Thinking... X.Xs"
pub fn thinking(state: &AppState) -> Vec<DisplayLine> {
    let elapsed = state.thinking_elapsed_secs()
        .map(|s| format!("⏳ Thinking... {:.1}s", s))
        .unwrap_or_else(|| "⏳ Thinking...".to_string());
    
    vec![
        DisplayLine {
            spans: vec![DisplaySpan { text: elapsed, color: Some(Color::DarkGray) }],
        },
        DisplayLine::empty(),
    ]
}

/// Thought duration (static): "⏳ Thought X.Xs"
pub fn thought_duration(state: &AppState) -> Vec<DisplayLine> {
    let duration = match state.thought_duration_secs() {
        Some(s) => format!("⏳ Thought {:.1}s", s),
        None => return vec![],
    };
    
    vec![
        DisplayLine {
            spans: vec![DisplaySpan { text: duration, color: Some(Color::DarkGray) }],
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
    
    pub fn is_thought(&self) -> bool {
        self.spans.iter().any(|s| s.text.contains("Thought"))
    }
    
    pub fn is_agent(&self) -> bool {
        self.spans.iter().any(|s| s.text.contains("Agent:"))
    }
}
