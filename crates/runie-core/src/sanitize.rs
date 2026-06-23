//! Defensive sanitization of message histories before sending to LLM providers.
//!
//! Repairs common edge cases:
//! - Empty assistant messages (no content, no tool calls)
//! - Dangling tool calls (assistant msg with tool_calls but no matching result)
//! - Orphan tool results (tool result without a matching tool call)
//! - Consecutive same-role messages (except tool results)
//! - Histories not starting with user or system
//! - Trailing whitespace in assistant content

use crate::message::{ChatMessage, Part, Role};

/// Apply all sanitization fixers in order.
///
/// Order matters: `remove_dangling_tool_calls` must run before
/// `remove_empty_assistant_messages` so that tool calls on empty assistant
/// messages are properly detected as dangling before the message is removed.
pub fn sanitize_messages(messages: &mut Vec<ChatMessage>) {
    remove_dangling_tool_calls(messages);
    remove_empty_assistant_messages(messages);
    remove_orphan_tool_results(messages);
    merge_consecutive_same_role(messages);
    ensure_starts_with_user_or_system(messages);
    trim_assistant_whitespace(messages);
}

/// Remove tool calls from assistant messages that have no matching tool result.
///
/// A tool call is "dangling" if the next message is NOT a tool result for it.
/// This must run before `remove_empty_assistant_messages` so we can detect
/// dangling calls before the assistant message is potentially removed.
fn remove_dangling_tool_calls(messages: &mut Vec<ChatMessage>) {
    let mut i = 0;
    while i < messages.len() {
        let msg = &messages[i];
        let has_tool_calls = !msg.tool_calls().is_empty();
        if msg.role == Role::Assistant && has_tool_calls {
            let next_is_tool_result = messages
                .get(i + 1)
                .map(|m| m.role == Role::Tool && m.tool_call_id.is_some())
                .unwrap_or(false);

            if !next_is_tool_result {
                messages[i].parts.retain(|p| !matches!(p, Part::ToolCall { .. }));
            }
        }
        i += 1;
    }
}

/// Remove assistant messages that have neither content nor tool calls.
fn remove_empty_assistant_messages(messages: &mut Vec<ChatMessage>) {
    messages.retain(|m| {
        if m.role == Role::Assistant {
            let has_content = !m.content().trim().is_empty();
            let has_tool_calls = !m.tool_calls().is_empty();
            has_content || has_tool_calls
        } else {
            true
        }
    });
}

/// Remove tool result messages that have no matching tool call in a preceding
/// assistant message.
fn remove_orphan_tool_results(messages: &mut Vec<ChatMessage>) {
    let mut seen_tool_ids = std::collections::HashSet::new();

    for msg in messages.iter() {
        if msg.role == Role::Assistant {
            for tc in msg.tool_calls() {
                if !tc.id.is_empty() {
                    seen_tool_ids.insert(tc.id.clone());
                }
            }
        }
    }

    messages.retain(|m| {
        if m.role == Role::Tool {
            m.tool_call_id
                .as_ref()
                .map(|id| seen_tool_ids.contains(id))
                .unwrap_or(false)
        } else {
            true
        }
    });
}

/// Merge consecutive messages with the same role (except tool results).
fn merge_consecutive_same_role(messages: &mut Vec<ChatMessage>) {
    let mut out: Vec<ChatMessage> = Vec::new();
    for mut msg in std::mem::take(messages) {
        if let Some(last) = out.last_mut() {
            if last.role == msg.role && msg.role != Role::Tool {
                let msg_content = msg.content();
                if !msg_content.trim().is_empty() {
                    if let Some(Part::Text { content: last_text }) = last.parts.last_mut() {
                        if !last_text.is_empty() {
                            last_text.push('\n');
                        }
                        last_text.push_str(&msg_content);
                    } else {
                        last.parts.push(Part::Text { content: msg_content });
                    }
                }
                // Merge tool calls
                for tc in msg.tool_calls() {
                    last.parts.push(Part::ToolCall { id: tc.id.clone(), name: tc.name.clone(), args: tc.args.clone() });
                }
                continue;
            }
        }
        out.push(msg);
    }
    *messages = out;
}

/// Prepend a user placeholder if the history does not start with user or system.
fn ensure_starts_with_user_or_system(messages: &mut Vec<ChatMessage>) {
    if messages
        .first()
        .map(|m| m.role == Role::User || m.role == Role::System)
        .unwrap_or(false)
    {
        return;
    }
    messages.insert(0, ChatMessage::user("[conversation start]".to_string()));
}

/// Trim trailing whitespace from assistant message content.
fn trim_assistant_whitespace(messages: &mut Vec<ChatMessage>) {
    for msg in messages.iter_mut() {
        if msg.role == Role::Assistant {
            if let Some(Part::Text { content }) = msg.parts.last_mut() {
                *content = content.trim_end().to_string();
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn user(content: &str) -> ChatMessage {
        ChatMessage::user(content.to_string())
    }
    fn assistant(content: &str) -> ChatMessage {
        ChatMessage::assistant(content.to_string())
    }
    fn tool(content: &str, id: &str) -> ChatMessage {
        ChatMessage::tool(content.to_string()).with_tool_call_id(id)
    }

    #[test]
    fn sanitize_removes_empty_assistant() {
        let mut msgs = vec![user("hello"), assistant(""), assistant("real reply")];
        sanitize_messages(&mut msgs);
        assert_eq!(msgs.len(), 2);
        assert!(msgs.iter().any(|m| m.content() == "real reply"));
    }

    #[test]
    fn sanitize_removes_dangling_tool_call() {
        let mut msgs = vec![
            assistant("call a tool"),
            user("ok"), // not a tool result
        ];
        msgs[0].parts.push(Part::ToolCall {
            id: "call_1".into(),
            name: "bash".into(),
            args: serde_json::json!({}),
        });
        sanitize_messages(&mut msgs);
        assert!(msgs[0].tool_calls().is_empty());
    }

    #[test]
    fn sanitize_keeps_matched_tool_call_and_result() {
        let mut msgs = vec![
            assistant("call a tool"),
            tool("result", "call_1"),
        ];
        msgs[0].parts.push(Part::ToolCall {
            id: "call_1".into(),
            name: "bash".into(),
            args: serde_json::json!({}),
        });
        sanitize_messages(&mut msgs);
        // Placeholder is prepended, so original assistant is at index 1, tool result at index 2
        assert_eq!(msgs[0].role, Role::User); // placeholder
        assert_eq!(msgs[1].role, Role::Assistant);
        assert!(!msgs[1].tool_calls().is_empty());
        assert_eq!(msgs[2].role, Role::Tool);
    }

    #[test]
    fn sanitize_removes_orphan_tool_result() {
        let mut msgs = vec![user("hello"), tool("orphan result", "unknown_id")];
        sanitize_messages(&mut msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, Role::User);
    }

    #[test]
    fn sanitize_merges_consecutive_user() {
        let mut msgs = vec![user("part 1"), user("part 2"), assistant("reply")];
        sanitize_messages(&mut msgs);
        assert_eq!(msgs.len(), 2);
        assert!(msgs[0].content().contains("part 1"));
        assert!(msgs[0].content().contains("part 2"));
    }

    #[test]
    fn sanitize_prepends_placeholder_when_first_is_assistant() {
        let mut msgs = vec![assistant("hi")];
        sanitize_messages(&mut msgs);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, Role::User);
        assert_eq!(msgs[1].role, Role::Assistant);
    }

    #[test]
    fn sanitize_trims_assistant_whitespace() {
        let mut msgs = vec![user("hello"), assistant("reply   \n  \t  ")];
        sanitize_messages(&mut msgs);
        assert_eq!(msgs[1].content(), "reply");
    }

    #[test]
    fn sanitize_pipeline_is_idempotent() {
        let mut msgs = vec![
            user("hello"),
            assistant("reply"),
        ];
        let first = msgs.clone();
        sanitize_messages(&mut msgs);
        sanitize_messages(&mut msgs);
        sanitize_messages(&mut msgs);
        assert_eq!(msgs, first);
    }
}
