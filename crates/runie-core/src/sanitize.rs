//! Defensive sanitization of message histories before sending to LLM providers.
//!
//! Repairs common edge cases:
//! - Empty assistant messages (no content, no tool calls)
//! - Dangling tool calls (assistant msg with tool_calls but no matching result)
//! - Orphan tool results (tool result without a matching tool call)
//! - Consecutive same-role messages (except tool results)
//! - Histories not starting with user or system
//! - Leading/trailing whitespace in assistant content

use crate::message::{ChatMessage, Part, Role};
use thiserror::Error;

/// Error produced by a sanitizer when it removes or repairs content.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SanitizeError {
    /// A tool call had no matching tool result.
    #[error("dangling tool call: {tool_call_id}")]
    DanglingToolCall { tool_call_id: String },
    /// A tool result had no matching tool call.
    #[error("orphan tool result: {tool_call_id}")]
    OrphanToolResult { tool_call_id: String },
    /// A message was removed entirely.
    #[error("removed {role:?} message: {reason}")]
    RemovedMessage { role: Role, reason: &'static str },
}

/// Apply all sanitization fixers in order, returning collected errors.
///
/// Order matters: `remove_dangling_tool_calls` must run before
/// `remove_empty_assistant_messages` so that tool calls on empty assistant
/// messages are properly detected as dangling before the message is removed.
pub fn sanitize_messages(messages: &mut Vec<ChatMessage>) -> Vec<SanitizeError> {
    let mut errors: Vec<SanitizeError> = Vec::new();
    let mut cleaned_indices = Vec::new();
    errors.extend(remove_dangling_tool_calls(messages, &mut cleaned_indices));
    errors.extend(remove_empty_assistant_messages(messages, &cleaned_indices));
    errors.extend(remove_orphan_tool_results(messages));
    errors.extend(merge_consecutive_same_role(messages));
    errors.extend(ensure_starts_with_user_or_system(messages));
    errors.extend(trim_assistant_whitespace(messages));
    errors
}

// ─── Sanitizers ────────────────────────────────────────────────────────────────

/// Remove tool calls from assistant messages that have no matching tool result.
fn remove_dangling_tool_calls(
    messages: &mut [ChatMessage],
    cleaned_indices: &mut Vec<usize>,
) -> Vec<SanitizeError> {
    let mut errors = Vec::new();
    for i in 0..messages.len() {
        if messages[i].role != Role::Assistant {
            continue;
        }
        let tool_call_ids: Vec<String> = messages[i]
            .tool_calls()
            .iter()
            .filter(|tc| !tc.id.is_empty())
            .map(|tc| tc.id.clone())
            .collect();
        if tool_call_ids.is_empty() {
            continue;
        }
        let has_result = messages[i + 1..].iter().any(|m| {
            m.tool_call_id
                .as_ref()
                .map(|id| tool_call_ids.contains(id))
                .unwrap_or(false)
        });
        if !has_result {
            errors.extend(remove_dangling_from_message(
                &mut messages[i],
                tool_call_ids,
            ));
            cleaned_indices.push(i);
        }
    }
    errors
}

fn remove_dangling_from_message(
    msg: &mut ChatMessage,
    dangling_ids: Vec<String>,
) -> Vec<SanitizeError> {
    let mut errors = Vec::new();
    let removed: Vec<_> = msg
        .tool_calls()
        .into_iter()
        .filter(|tc| dangling_ids.contains(&tc.id))
        .collect();
    msg.parts
        .retain(|p| !matches!(p, Part::ToolCall { id, .. } if dangling_ids.contains(id)));
    for tc in removed {
        errors.push(SanitizeError::DanglingToolCall {
            tool_call_id: tc.id,
        });
    }
    errors
}

/// Remove assistant messages that have neither content nor tool calls.
/// Skips messages that had dangling tool calls removed (already cleaned).
fn remove_empty_assistant_messages(
    messages: &mut Vec<ChatMessage>,
    cleaned_indices: &[usize],
) -> Vec<SanitizeError> {
    let mut errors = Vec::new();
    let cleaned: std::collections::HashSet<usize> = cleaned_indices.iter().cloned().collect();
    let mut retain_mask = Vec::with_capacity(messages.len());
    for (i, m) in messages.iter().enumerate() {
        let keep = if m.role == Role::Assistant && !cleaned.contains(&i) {
            let has_content = !m.content().trim().is_empty();
            let has_tcs = !m.tool_calls().is_empty();
            has_content || has_tcs
        } else {
            true
        };
        retain_mask.push(keep);
        if !keep {
            errors.push(SanitizeError::RemovedMessage {
                role: Role::Assistant,
                reason: "empty message",
            });
        }
    }
    let mut write_idx = 0;
    for (read_idx, keep) in retain_mask.into_iter().enumerate() {
        if keep {
            if write_idx != read_idx {
                messages.swap(write_idx, read_idx);
            }
            write_idx += 1;
        }
    }
    messages.truncate(write_idx);
    errors
}

/// Remove tool result messages that have no matching tool call in a preceding assistant message.
fn remove_orphan_tool_results(messages: &mut Vec<ChatMessage>) -> Vec<SanitizeError> {
    let mut errors = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    for msg in messages.iter() {
        if msg.role == Role::Assistant {
            for tc in msg.tool_calls() {
                if !tc.id.is_empty() {
                    seen_ids.insert(tc.id.clone());
                }
            }
        }
    }
    let before = messages.len();
    messages.retain(|m| {
        m.role != Role::Tool
            || m.tool_call_id
                .as_ref()
                .map(|id| seen_ids.contains(id))
                .unwrap_or(false)
    });
    let removed = before - messages.len();
    for _ in 0..removed {
        errors.push(SanitizeError::OrphanToolResult {
            tool_call_id: "<unknown>".to_owned(),
        });
    }
    errors
}

/// Merge consecutive messages with the same role (except tool results). Uses `\n\n` to separate.
fn merge_consecutive_same_role(messages: &mut Vec<ChatMessage>) -> Vec<SanitizeError> {
    let mut out: Vec<ChatMessage> = Vec::new();
    for msg in std::mem::take(messages) {
        if let Some(last) = out.last_mut() {
            if last.role == msg.role && msg.role != Role::Tool {
                let content = msg.content();
                if !content.trim().is_empty() {
                    if let Some(Part::Text { content: last_text }) = last.parts.last_mut() {
                        if !last_text.is_empty() {
                            last_text.push_str("\n\n");
                        }
                        last_text.push_str(&content);
                    } else {
                        last.parts.push(Part::Text { content });
                    }
                }
                for tc in msg.tool_calls() {
                    last.parts.push(Part::ToolCall {
                        id: tc.id,
                        name: tc.name,
                        args: tc.args,
                    });
                }
                continue;
            }
        }
        out.push(msg);
    }
    *messages = out;
    Vec::new()
}

/// Prepend a system placeholder if the history does not start with user or system.
fn ensure_starts_with_user_or_system(messages: &mut Vec<ChatMessage>) -> Vec<SanitizeError> {
    if messages
        .first()
        .map(|m| m.role == Role::User || m.role == Role::System)
        .unwrap_or(false)
    {
        return Vec::new();
    }
    messages.insert(0, ChatMessage::system("Continue."));
    Vec::new()
}

/// Trim leading and trailing whitespace from assistant message content.
fn trim_assistant_whitespace(messages: &mut [ChatMessage]) -> Vec<SanitizeError> {
    for msg in messages.iter_mut() {
        if msg.role == Role::Assistant {
            if let Some(Part::Text { content }) = msg.parts.last_mut() {
                *content = content.trim().to_owned();
            }
        }
    }
    Vec::new()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn user(s: &str) -> ChatMessage {
        ChatMessage::user(s)
    }
    fn assistant(s: &str) -> ChatMessage {
        ChatMessage::assistant(s)
    }
    fn tool(s: &str, id: &str) -> ChatMessage {
        ChatMessage::tool(s).with_tool_call_id(id)
    }

    #[test]
    fn sanitize_removes_empty_assistant() {
        let mut msgs = vec![user("hello"), assistant(""), assistant("real reply")];
        let errs = sanitize_messages(&mut msgs);
        assert_eq!(msgs.len(), 2);
        assert!(msgs.iter().any(|m| m.content() == "real reply"));
        assert!(errs.iter().any(|e| matches!(
            e,
            SanitizeError::RemovedMessage {
                role: Role::Assistant,
                ..
            }
        )));
    }

    #[test]
    fn sanitize_removes_dangling_tool_call() {
        let mut msgs = vec![assistant("call a tool"), user("ok")];
        msgs[0].parts.push(Part::ToolCall {
            id: "c1".into(),
            name: "bash".into(),
            args: serde_json::json!({}),
        });
        let errs = sanitize_messages(&mut msgs);
        assert!(msgs[0].tool_calls().is_empty());
        assert!(errs.iter().any(|e| matches!(e, SanitizeError::DanglingToolCall { tool_call_id } if tool_call_id == "c1")));
    }

    #[test]
    fn sanitize_keeps_matched_tool_call_and_result() {
        let mut msgs = vec![assistant("call a tool"), tool("result", "c1")];
        msgs[0].parts.push(Part::ToolCall {
            id: "c1".into(),
            name: "bash".into(),
            args: serde_json::json!({}),
        });
        sanitize_messages(&mut msgs);
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[1].role, Role::Assistant);
        assert!(!msgs[1].tool_calls().is_empty());
        assert_eq!(msgs[2].role, Role::Tool);
    }

    #[test]
    fn sanitize_removes_orphan_tool_result() {
        let mut msgs = vec![user("hi"), tool("orphan result", "orphan")];
        let errs = sanitize_messages(&mut msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, Role::User);
        assert!(errs
            .iter()
            .any(|e| matches!(e, SanitizeError::OrphanToolResult { .. })));
    }

    #[test]
    fn sanitize_merges_consecutive_user() {
        let mut msgs = vec![user("part 1"), user("part 2"), assistant("reply")];
        sanitize_messages(&mut msgs);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content(), "part 1\n\npart 2");
        assert_eq!(msgs[1].role, Role::Assistant);
    }

    #[test]
    fn sanitize_prepends_placeholder_when_first_is_assistant() {
        let mut msgs = vec![assistant("hi")];
        sanitize_messages(&mut msgs);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, Role::System);
        assert_eq!(msgs[0].content(), "Continue.");
        assert_eq!(msgs[1].role, Role::Assistant);
    }

    #[test]
    fn sanitize_trims_assistant_whitespace() {
        let mut msgs = vec![user("hello"), assistant("  hi  ")];
        sanitize_messages(&mut msgs);
        assert_eq!(msgs[1].content(), "hi");
    }

    #[test]
    fn sanitize_pipeline_is_idempotent() {
        let mut msgs = vec![user("hello"), assistant("reply")];
        let first = msgs.clone();
        sanitize_messages(&mut msgs);
        sanitize_messages(&mut msgs);
        assert_eq!(msgs, first);
    }
}
