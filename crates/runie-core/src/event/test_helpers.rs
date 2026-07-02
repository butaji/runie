//! Test helpers for creating Event variants with default fields.

use super::Event;

/// Create a Response event with default durable fields.
pub fn response(id: impl Into<String>, content: impl Into<String>) -> Event {
    Event::response(id, content)
}

/// Create a ToolEnd event with default input field.
pub fn tool_end(id: impl Into<String>, duration_secs: f64, output: impl Into<String>) -> Event {
    Event::tool_end(id, duration_secs, output)
}
