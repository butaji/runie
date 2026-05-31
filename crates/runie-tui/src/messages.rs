//! Unified message registry for all UI text content.
//!
//! Rules:
//! - ALL user-visible text goes through MessageRegistry
//! - Status text ALWAYS Title Case
//! - Error text ALWAYS Title Case + description
//! - Tool names ALWAYS lowercase
//! - Timing ALWAYS "X.Xs" format

use crate::glyphs;

/// Central registry for ALL UI messages
pub struct MessageRegistry;

impl MessageRegistry {
    // ─── Status messages (Title Case) ───────────────────────────────────────

    /// Agent is thinking/reasoning
    pub fn status_thinking() -> &'static str {
        "Thinking"
    }

    /// Agent is working (tool execution)
    pub fn status_running() -> &'static str {
        "Running"
    }

    /// An error occurred
    pub fn status_error() -> &'static str {
        "Error"
    }

    /// Agent is idle (no active job)
    pub fn status_idle() -> &'static str {
        "Idle"
    }

    // ─── Error messages (Title Case + description) ──────────────────────────

    /// No API key configured for provider
    pub fn error_no_api_key(provider: &str) -> String {
        format!("No API key configured for {}", provider)
    }

    /// Unauthorized access
    pub fn error_unauthorized() -> &'static str {
        "Unauthorized"
    }

    /// Generic error with message
    pub fn error_with_message(message: &str) -> String {
        message.to_string()
    }

    // ─── Tool messages (lowercase tool names) ────────────────────────────────

    /// Tool execution started
    pub fn tool_running(name: &str) -> String {
        format!("running {}", name.to_lowercase())
    }

    /// Tool execution completed
    pub fn tool_complete(name: &str) -> String {
        name.to_string()
    }

    // ─── Timing messages (X.Xs format) ──────────────────────────────────────

    /// Turn completed with duration
    pub fn turn_completed(seconds: f32) -> String {
        format!("Turn completed in {:.1}s", seconds)
    }

    /// Thought duration indicator
    pub fn thought_duration(seconds: f32) -> String {
        format!("Thought for {:.1}s", seconds)
    }

    // ─── Status bar formatting ──────────────────────────────────────────────

    /// Format status header with elapsed time
    pub fn status_with_time(header: &str, seconds: u64) -> String {
        if seconds < 60 {
            format!("{} ({}s)", header, seconds)
        } else if seconds < 3600 {
            format!("{} ({}m {:02}s)", header, seconds / 60, seconds % 60)
        } else {
            format!(
                "{} ({}h {:02}m {:02}s)",
                header,
                seconds / 3600,
                (seconds % 3600) / 60,
                seconds % 60
            )
        }
    }

    /// Format elapsed time as "Xs", "Xm YYs", or "Xh YYm ZZs"
    pub fn format_elapsed(seconds: u64) -> String {
        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {:02}s", seconds / 60, seconds % 60)
        } else {
            format!(
                "{}h {:02}m {:02}s",
                seconds / 3600,
                (seconds % 3600) / 60,
                seconds % 60
            )
        }
    }

    // ─── Global tags formatting ──────────────────────────────────────────────

    /// Running status for global tags: "⣾ {status} [turn: {time}]"
    pub fn global_tags_running(status: &str, time: &str, tokens: u64) -> String {
        format!("{} {} [turn: {}] [⇣{}]", glyphs::SPINNER_FRAMES[0], status, time, tokens)
    }

    /// Idle status for global tags: "{model} | {tokens} tok | ${cost}"
    pub fn global_tags_idle(model: &str, tokens: u64, cost: f64) -> String {
        if tokens > 0 {
            format!("{} | {} tok | ${:.4}", model, tokens, cost)
        } else {
            model.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_messages_title_case() {
        assert_eq!(MessageRegistry::status_thinking(), "Thinking");
        assert_eq!(MessageRegistry::status_running(), "Running");
        assert_eq!(MessageRegistry::status_error(), "Error");
        assert_eq!(MessageRegistry::status_idle(), "Idle");
    }

    #[test]
    fn test_error_no_api_key() {
        let msg = MessageRegistry::error_no_api_key("OpenAI");
        assert!(msg.contains("OpenAI"));
        assert!(msg.starts_with("No API key configured"));
    }

    #[test]
    fn test_tool_running_lowercase() {
        let msg = MessageRegistry::tool_running("Bash");
        assert_eq!(msg, "running bash");
    }

    #[test]
    fn test_tool_complete_preserves_name() {
        let msg = MessageRegistry::tool_complete("Bash");
        assert_eq!(msg, "Bash");
    }

    #[test]
    fn test_turn_completed_format() {
        let msg = MessageRegistry::turn_completed(1.5);
        assert!(msg.contains("1.5s"));
        assert!(msg.contains("Turn completed"));
    }

    #[test]
    fn test_thought_duration_format() {
        let msg = MessageRegistry::thought_duration(2.3);
        assert!(msg.contains("2.3s"));
        assert!(msg.contains("Thought for"));
    }

    #[test]
    fn test_format_elapsed() {
        assert_eq!(MessageRegistry::format_elapsed(30), "30s");
        assert_eq!(MessageRegistry::format_elapsed(90), "1m 30s");
        assert_eq!(MessageRegistry::format_elapsed(3661), "1h 01m 01s");
    }

    #[test]
    fn test_status_with_time() {
        assert_eq!(
            MessageRegistry::status_with_time("Thinking", 5),
            "Thinking (5s)"
        );
        assert_eq!(
            MessageRegistry::status_with_time("Running", 90),
            "Running (1m 30s)"
        );
    }
}
