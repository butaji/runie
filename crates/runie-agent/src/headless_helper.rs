//! Headless CLI helper — shared setup for runie-print, runie-json, runie-server.
//!
//! These functions extract the common runtime setup that all three headless
//! binaries share: system-prompt construction, message-list building, and
//! `HeadlessCliOptions` with the common defaults (`execute_tools: true`,
//! `max_tool_rounds: 5`).

use std::sync::Arc;

use runie_core::message::ChatMessage;
use runie_core::permissions::build_sink as core_sink;
use runie_core::prompts::{build_system_prompt as core_system_prompt, DEFAULT_PROMPT, DEFAULT_TOOLS};

use crate::HeadlessCliOptions;

/// Build a system prompt string with the default tools and harness context.
pub fn build_system_prompt() -> String {
    core_system_prompt(DEFAULT_PROMPT, DEFAULT_TOOLS, false, "")
}

/// Build a chat message list from a single user prompt.
pub fn build_messages(user_prompt: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage::system(build_system_prompt()),
        ChatMessage::user(user_prompt.to_string()),
    ]
}

/// Build `HeadlessCliOptions` with common defaults for headless mode.
///
/// - `execute_tools: true` — run tools by default
/// - `max_tool_rounds: 5` — conservative round limit for CLI use
/// - `on_chunk: None` — caller supplies their own chunk handler
pub fn build_options(
    on_chunk: Option<Box<dyn FnMut(&str) + Send>>,
) -> HeadlessCliOptions {
    HeadlessCliOptions {
        execute_tools: true,
        max_tool_rounds: 5,
        on_chunk,
    }
}

/// Build a permission sink for headless mode.
///
/// In yolo mode, destructive tools are auto-approved.
pub fn build_sink(yolo: bool) -> Arc<dyn runie_core::permissions::ApprovalSink> {
    core_sink(yolo)
}

/// Run a headless CLI turn with the common defaults.
pub async fn run_headless(
    messages: Vec<ChatMessage>,
    yolo: bool,
    on_chunk: Option<Box<dyn FnMut(&str) + Send>>,
) -> anyhow::Result<crate::HeadlessResult> {
    let sink = build_sink(yolo);
    let opts = build_options(on_chunk);
    crate::run_headless_cli(None, None, messages, sink, opts).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt_is_non_empty() {
        let prompt = build_system_prompt();
        assert!(!prompt.is_empty());
    }

    #[test]
    fn test_build_messages_has_system_and_user() {
        let msgs = build_messages("hello");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, runie_core::message::Role::System);
        assert_eq!(msgs[1].role, runie_core::message::Role::User);
        assert_eq!(msgs[1].content(), "hello");
    }

    #[test]
    fn test_build_options_defaults() {
        let opts = build_options(None);
        assert!(opts.execute_tools);
        assert_eq!(opts.max_tool_rounds, 5);
        assert!(opts.on_chunk.is_none());
    }

    #[test]
    fn test_build_sink_returns_something() {
        let sink = build_sink(false);
        let _ = sink;
    }
}
