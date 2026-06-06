//! All static labels and text constants
//! PANEL_CHAT and PANEL_INPUT are defined in model.rs

pub const PREFIX_USER: &str = "You: ";
pub const PREFIX_AGENT: &str = "Agent: ";

pub const SPINNER: &str = "⠋";
pub const THINKING_LOADING: &str = "Thinking...";

pub fn thinking_with_time(seconds: f64) -> String {
    format!("{} Thinking... {:.1}s", SPINNER, seconds)
}

pub fn thought_with_time(seconds: f64) -> String {
    format!("◆ Thought {:.1}s", seconds)
}

pub fn tool_running(name: &str) -> String {
    format!("{} Running {}...", SPINNER, name)
}

pub fn tool_done(name: &str, seconds: f64) -> String {
    format!("◆ Ran {} {:.1}s", name, seconds)
}
