//! All static labels and text constants
//! Centralized for easy localization and maintenance

/// Chat panel title
pub const PANEL_CHAT: &str = " Chat ";

/// Input panel title  
pub const PANEL_INPUT: &str = " Input ";

/// User message prefix
pub const PREFIX_USER: &str = "You: ";

/// Agent message prefix
pub const PREFIX_AGENT: &str = "Agent: ";

/// Thinking indicator (no time yet)
pub const THINKING_LOADING: &str = "⏳ Thinking...";

/// Thinking indicator with time
pub fn thinking_with_time(seconds: f64) -> String {
    format!("⏳ Thinking... {:.1}s", seconds)
}

/// Thought duration (after completion)
pub fn thought_with_time(seconds: f64) -> String {
    format!("⏳ Thought {:.1}s", seconds)
}
