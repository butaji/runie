//! Message validation utilities.

use std::collections::HashSet;

use super::chat_message::ChatMessage;
use super::parts::Part;
use super::role::Role;

/// Validate a single message for structural validity.
/// Returns `None` if valid, `Some(reason)` if not.
pub fn validate_message(msg: &ChatMessage) -> Option<&'static str> {
    match msg.role {
        Role::Assistant => {
            let has_text = msg
                .parts
                .iter()
                .any(|p| matches!(p, Part::Text { content } if !content.is_empty()));
            let has_tc = msg.parts.iter().any(|p| matches!(p, Part::ToolCall { .. }));
            if !has_text && !has_tc {
                return Some("assistant message has no text or tool calls");
            }
        }
        Role::Tool => {
            if msg.tool_call_id.is_none() {
                return Some("tool message missing tool_call_id");
            }
        }
        Role::Thought => {
            let has_text = msg
                .parts
                .iter()
                .any(|p| matches!(p, Part::Text { content } if !content.is_empty()));
            if !has_text {
                return Some("thought message has no content");
            }
        }
        Role::User | Role::System | Role::TurnComplete => {}
    }
    None
}

/// Validate a sequence of messages for structural validity.
///
/// Checks:
/// - Every tool message has a matching tool call id in a preceding assistant message
/// - Assistant messages are not empty (no text, no tool calls)
///
/// Use this after building a message history, not instead of per-message construction.
pub fn validate_messages(messages: &[ChatMessage]) -> Vec<SanitizeError> {
    let mut errors = Vec::new();

    // Collect all valid tool call ids
    let tool_call_ids: HashSet<_> = messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .flat_map(|m| m.tool_calls())
        .filter(|tc| !tc.id.is_empty())
        .map(|tc| tc.id)
        .collect();

    for msg in messages {
        // Check tool message has matching tool call
        if msg.role == Role::Tool {
            if let Some(ref id) = msg.tool_call_id {
                if !tool_call_ids.contains(id) {
                    errors.push(SanitizeError::OrphanToolResult {
                        tool_call_id: id.clone(),
                    });
                }
            } else {
                errors.push(SanitizeError::OrphanToolResult {
                    tool_call_id: "<missing>".to_owned(),
                });
            }
        }

        // Check assistant not empty
        if msg.role == Role::Assistant {
            let has_text = !msg.content().trim().is_empty();
            let has_tc = !msg.tool_calls().is_empty();
            if !has_text && !has_tc {
                errors.push(SanitizeError::RemovedMessage {
                    role: Role::Assistant,
                    reason: "empty message",
                });
            }
        }
    }

    errors
}

/// Validation errors for message sequences.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SanitizeError {
    #[error("dangling tool call: {tool_call_id}")]
    DanglingToolCall { tool_call_id: String },
    #[error("orphan tool result: {tool_call_id}")]
    OrphanToolResult { tool_call_id: String },
    #[error("removed {role:?} message: {reason}")]
    RemovedMessage { role: Role, reason: &'static str },
}
