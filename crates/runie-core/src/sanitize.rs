//! Defensive sanitization of message histories before sending to LLM providers.
//!
//! This module provides **validation-only** sanitization. Use `ChatMessageBuilder`
//! to construct valid individual messages. Use `validate_messages` to check
//! a sequence for dangling tool calls and orphan results. Use `trim_messages`
//! to normalize whitespace.
//!
//! Historical fix-up logic (removing empty messages, merging consecutive roles,
//! adding placeholders) has been removed — those issues should be prevented at
//! construction time, not repaired post-hoc.

use crate::proto::message::{validate_messages, SanitizeError};

/// Validate a message sequence and normalize whitespace.
///
/// Order: validate first, then trim — so trimming doesn't mask validation errors.
pub fn sanitize_messages(messages: &mut Vec<crate::message::ChatMessage>) -> Vec<SanitizeError> {
    let errors = validate_messages(messages);
    trim_messages(messages);
    errors
}

/// Normalize whitespace: trim leading/trailing space from assistant text parts.
fn trim_messages(messages: &mut [crate::message::ChatMessage]) {
    for msg in messages.iter_mut() {
        if msg.role == crate::message::Role::Assistant {
            if let Some(crate::message::Part::Text { content }) = msg.parts.last_mut() {
                *content = content.trim().to_owned();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::message::{ChatMessage, Part, Role};
    use crate::proto::message::{validate_messages, SanitizeError};
    use crate::sanitize::sanitize_messages;

    fn user(s: &str) -> ChatMessage {
        ChatMessage::user(s)
    }
    fn assistant(s: &str) -> ChatMessage {
        ChatMessage::assistant(s)
    }
    fn tool(s: &str, id: &str) -> ChatMessage {
        ChatMessage::tool(s).with_tool_call_id(id)
    }

    /// Tool result with matching tool call id is valid.
    #[test]
    fn validate_passes_matched_tool_call_and_result() {
        let mut msgs = vec![assistant("call a tool"), tool("result", "c1")];
        msgs[0].parts.push(Part::ToolCall {
            id: "c1".into(),
            name: "bash".into(),
            args: serde_json::json!({}),
        });
        let errs = validate_messages(&msgs);
        assert!(errs.is_empty(), "matched tool call/result should pass: {:?}", errs);
    }

    /// Orphan tool result (no matching tool call) is detected.
    #[test]
    fn validate_detects_orphan_tool_result() {
        let mut msgs = vec![user("hi"), tool("orphan result", "orphan")];
        let errs = validate_messages(&msgs);
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|e| matches!(e, SanitizeError::OrphanToolResult { .. })));
    }

    /// Empty assistant message is detected.
    #[test]
    fn validate_detects_empty_assistant() {
        let msgs = vec![user("hello"), assistant(""), assistant("real reply")];
        let errs = validate_messages(&msgs);
        // Only the middle (truly empty) message should be flagged
        let removed: Vec<_> = errs.iter()
            .filter_map(|e| match e { SanitizeError::RemovedMessage { role: Role::Assistant, .. } => Some(()), _ => None })
            .collect();
        assert_eq!(removed.len(), 1, "only one empty assistant should be flagged");
    }

    /// Assistant message with only tool calls (no text) is valid.
    #[test]
    fn validate_assistant_with_only_tool_calls_is_valid() {
        let mut msgs = vec![user("hi")];
        msgs.push(ChatMessage {
            role: Role::Assistant,
            timestamp: 0.0,
            id: String::new(),
            provider: String::new(),
            metadata: Default::default(),
            tool_call_id: None,
            provider_metadata: None,
            parts: vec![Part::ToolCall {
                id: "c1".into(),
                name: "bash".into(),
                args: serde_json::json!({}),
            }],
        });
        let errs = validate_messages(&msgs);
        assert!(errs.is_empty(), "assistant with tool calls but no text is valid: {:?}", errs);
    }

    /// Trim removes leading/trailing whitespace from assistant messages.
    #[test]
    fn trim_removes_assistant_whitespace() {
        let mut msgs = vec![user("hello"), assistant("  hi  ")];
        let errs = sanitize_messages(&mut msgs);
        assert!(errs.is_empty());
        assert_eq!(msgs[1].content(), "hi");
    }

    /// Validation errors are returned without modifying the sequence.
    #[test]
    fn sanitize_returns_errors_without_mutating() {
        let mut msgs = vec![user("hi"), tool("orphan", "orphan")];
        let errs = sanitize_messages(&mut msgs);
        assert!(!errs.is_empty());
        // Messages should still be intact (not removed by sanitizer)
        assert_eq!(msgs.len(), 2);
    }

    /// Sanitize is idempotent on already-clean messages.
    #[test]
    fn sanitize_pipeline_is_idempotent() {
        let mut msgs = vec![user("hello"), assistant("reply")];
        let first = msgs.clone();
        sanitize_messages(&mut msgs);
        sanitize_messages(&mut msgs);
        assert_eq!(msgs, first);
    }
}
