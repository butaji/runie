//! Defensive normalization of chat messages before OpenAI serialization.
//!
//! OpenAI-compatible endpoints reject malformed histories such as orphan
//! `role: "tool"` messages without a matching `tool_call_id`, consecutive
//! messages with the same role, or histories that do not start with a user
//! or system message. This module repairs the most common issues.

use runie_core::message::{ChatMessage, Role};

/// Normalize a message list for an OpenAI-compatible request.
pub fn normalize_messages(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let messages = strip_provider_metadata(messages);
    let messages = merge_consecutive_same_role(messages);
    ensure_user_or_system_first(messages)
}

fn strip_provider_metadata(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    messages
        .into_iter()
        .map(|mut m| {
            m.provider_metadata = None;
            m
        })
        .collect()
}

fn merge_consecutive_same_role(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let mut out: Vec<ChatMessage> = Vec::new();
    for msg in messages {
        if let Some(last) = out.last_mut() {
            if last.role == msg.role && last.role != Role::Tool {
                if !msg.content.is_empty() {
                    last.content.push('\n');
                    last.content.push_str(&msg.content);
                }
                last.tool_calls.extend(msg.tool_calls);
                continue;
            }
        }
        out.push(msg);
    }
    out
}

fn ensure_user_or_system_first(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    if messages
        .first()
        .map(|m| m.role != Role::Tool)
        .unwrap_or(true)
    {
        return messages;
    }
    let mut out = vec![ChatMessage::user("[conversation start]".to_string())];
    out.extend(messages);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_provider_metadata() {
        let mut msg = ChatMessage::user("hi".to_string());
        msg.provider_metadata = Some(serde_json::json!({"sig": "abc"}));
        let out = normalize_messages(vec![msg]);
        assert!(out[0].provider_metadata.is_none());
    }

    #[test]
    fn merges_consecutive_same_role_messages() {
        let messages = vec![
            ChatMessage::assistant("part 1".to_string()),
            ChatMessage::assistant("part 2".to_string()),
            ChatMessage::user("ok".to_string()),
        ];
        let out = normalize_messages(messages);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].content, "part 1\npart 2");
        assert_eq!(out[1].content, "ok");
    }

    #[test]
    fn does_not_merge_consecutive_tool_results() {
        let messages = vec![
            ChatMessage::system("sys".to_string()),
            ChatMessage::tool("result 1".to_string()).with_tool_call_id("call_1"),
            ChatMessage::tool("result 2".to_string()).with_tool_call_id("call_2"),
        ];
        let out = normalize_messages(messages);
        assert_eq!(out.len(), 3);
        assert_eq!(out[1].tool_call_id, Some("call_1".to_string()));
        assert_eq!(out[2].tool_call_id, Some("call_2".to_string()));
    }

    #[test]
    fn injects_user_placeholder_when_history_starts_with_tool() {
        let messages = vec![ChatMessage::tool("result".to_string())];
        let out = normalize_messages(messages);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].role, Role::User);
        assert_eq!(out[1].role, Role::Tool);
    }

    #[test]
    fn does_not_inject_placeholder_for_assistant_first() {
        let messages = vec![ChatMessage::assistant("hi".to_string())];
        let out = normalize_messages(messages);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].role, Role::Assistant);
    }
}
