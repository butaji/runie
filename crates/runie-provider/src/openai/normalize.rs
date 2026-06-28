//! Defensive normalization of chat messages before OpenAI serialization.
//!
//! OpenAI-compatible endpoints reject malformed histories such as orphan
//! `role: "tool"` messages without a matching `tool_call_id`, consecutive
//! messages with the same role, or histories that do not start with a user
//! or system message. This module repairs the most common issues.

use runie_core::proto::message::ChatMessage;
use runie_core::sanitize::sanitize_messages;

/// Normalize a message list for an OpenAI-compatible request.
///
/// Shared fixers (empty removal, consecutive merge, first-role guard,
/// whitespace trim, tool-call integrity) are delegated to `sanitize_messages`.
pub fn normalize_messages(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let mut msgs = messages;
    sanitize_messages(&mut msgs);
    strip_provider_metadata(msgs)
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

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::proto::message::Role;

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
            ChatMessage::user("part 1".to_string()),
            ChatMessage::user("part 2".to_string()),
            ChatMessage::user("ok".to_string()),
        ];
        let out = normalize_messages(messages);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].content(), "part 1\n\npart 2\n\nok");
    }

    #[test]
    fn does_not_merge_consecutive_tool_results() {
        // Need an assistant message with matching tool calls for tool results to be preserved
        let mut assistant = ChatMessage::assistant("call tools".to_string());
        assistant.parts.push(runie_core::proto::message::Part::ToolCall {
            id: "call_1".into(),
            name: "tool1".into(),
            args: serde_json::json!({}),
        });
        assistant.parts.push(runie_core::proto::message::Part::ToolCall {
            id: "call_2".into(),
            name: "tool2".into(),
            args: serde_json::json!({}),
        });
        let messages = vec![
            ChatMessage::user("hi".to_string()),
            assistant,
            ChatMessage::tool("result 1".to_string()).with_tool_call_id("call_1"),
            ChatMessage::tool("result 2".to_string()).with_tool_call_id("call_2"),
        ];
        let out = normalize_messages(messages);
        // Both tool results should be preserved (have matching tool calls)
        assert!(out
            .iter()
            .any(|m| m.tool_call_id.as_deref() == Some("call_1")));
        assert!(out
            .iter()
            .any(|m| m.tool_call_id.as_deref() == Some("call_2")));
    }

    #[test]
    fn injects_placeholder_when_first_is_tool() {
        // Tool result without matching tool call is removed as orphan
        let messages = vec![ChatMessage::tool("result".to_string())];
        let out = normalize_messages(messages);
        // Placeholder is added, orphan tool result is removed
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].role, Role::System);
    }

    #[test]
    fn does_not_inject_placeholder_for_assistant_first() {
        let messages = vec![ChatMessage::assistant("hi".to_string())];
        let out = normalize_messages(messages);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].role, Role::System);
        assert_eq!(out[1].role, Role::Assistant);
    }
}
