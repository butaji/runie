//! All static labels and text constants

pub const PANEL_CHAT: &str = " Chat ";
pub const PANEL_INPUT: &str = " Input ";
pub const PREFIX_USER: &str = "You: ";
pub const PREFIX_AGENT: &str = "Agent: ";
pub const THINKING_SPINNER: &str = "⠋";

pub const THINKING_LOADING: &str = "⠋ Though...";

pub fn thinking_with_time(seconds: f64) -> String {
    format!("⠋ Though... {:.1}s", seconds)
}

pub fn thought_with_time(seconds: f64) -> String {
    format!("◆ Though {:.1}s", seconds)
}
