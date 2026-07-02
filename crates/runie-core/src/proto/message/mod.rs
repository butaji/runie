//! Message types shared across the application.
//!
//! Key types:
//! - [`Role`] — participant role in a conversation
//! - [`MessageOrigin`] — origin of a message
//! - [`ChatMessage`] — a conversation message; built with [`ChatMessageBuilder`]
//! - [`ToolCall`] — a structured tool invocation
//! - [`Part`] — typed content blocks inside a message
//! - [`validate_message`] / [`validate_messages`] — structural validation

pub mod chat_message;
pub mod metadata;
pub mod parts;
pub mod role;
pub mod tool_call;
pub mod validation;

// ── Re-exports ────────────────────────────────────────────────────────────────

pub use parts::Part;
pub use role::{MessageOrigin, Role};
pub use tool_call::ToolCall;
pub use validation::{validate_message, validate_messages, SanitizeError};

// Re-export for compatibility with `crate::proto::message::now()`.
pub use chat_message::now;

// Back-compat re-exports for callers that use the old module path.
pub use chat_message::{ChatMessage, ChatMessageBuilder};
pub use metadata::MessageMetadata;

#[cfg(test)]
mod tests {
    use super::*;

    // ── Role tests ────────────────────────────────────────────────────────────

    #[test]
    fn role_as_str_matches_provider_expectations() {
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Assistant.as_str(), "assistant");
        assert_eq!(Role::System.as_str(), "system");
        assert_eq!(Role::Tool.as_str(), "tool");
    }

    // ── ChatMessage content tests ─────────────────────────────────────────────

    #[test]
    fn chat_message_content_getter_concatenates_text_parts() {
        let msg = ChatMessage {
            parts: vec![
                Part::Text { content: "a".into() },
                Part::Reasoning { content: "r".into() },
                Part::Text { content: "b".into() },
            ],
            ..Default::default()
        };
        assert_eq!(msg.content(), "ab");
    }

    #[test]
    fn chat_message_tool_calls_getter_extracts_from_parts() {
        let msg = ChatMessage {
            parts: vec![
                Part::Text { content: "hi".into() },
                Part::ToolCall { id: "c1".into(), name: "bash".into(), args: serde_json::json!({}) },
            ],
            ..Default::default()
        };
        let tcs = msg.tool_calls();
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0].id, "c1");
    }

    #[test]
    fn chat_message_new_creates_text_part() {
        let msg = ChatMessage::new(Role::User, "hello");
        assert_eq!(msg.content(), "hello");
        assert!(matches!(msg.parts[..], [Part::Text { content: ref c }] if c == "hello"));
    }

    #[test]
    fn chat_message_no_text_parts_returns_empty_content() {
        let msg = ChatMessage {
            parts: vec![Part::ToolCall { id: "c1".into(), name: "bash".into(), args: serde_json::json!({}) }],
            ..Default::default()
        };
        assert_eq!(msg.content(), "");
    }

    // ── Serialization round-trip tests ───────────────────────────────────────

    #[test]
    fn chat_message_round_trip_json() {
        let msg = ChatMessage {
            role: Role::User,
            timestamp: 1234567890.0,
            id: "msg-1".into(),
            provider: "openai".into(),
            metadata: MessageMetadata::default(),
            tool_call_id: None,
            provider_metadata: None,
            parts: vec![Part::Text { content: "hello".into() }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, Role::User);
        assert_eq!(parsed.content(), "hello");
        assert_eq!(parsed.id, "msg-1");
    }

    #[test]
    fn chat_message_serializes_round_trip() {
        let msg = ChatMessage {
            role: Role::Assistant,
            timestamp: 1.0,
            id: "a1".into(),
            parts: vec![
                Part::Text { content: "hello".into() },
                Part::ToolCall { id: "call_1".into(), name: "list_dir".into(), args: serde_json::json!({"path": "."}) },
            ],
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, Role::Assistant);
        assert_eq!(parsed.content(), "hello");
        assert_eq!(parsed.tool_calls().len(), 1);
        assert_eq!(parsed.tool_calls()[0].name, "list_dir");
    }

    #[test]
    fn canonical_tool_call_round_trips_through_json() {
        let tc = ToolCall::new("call_abc", "bash", serde_json::json!({"cmd": "ls -la"}));
        let json = serde_json::to_string(&tc).unwrap();
        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "call_abc");
        assert_eq!(parsed.name, "bash");
        assert_eq!(parsed.args["cmd"], "ls -la");
    }

    // ── ChatMessageBuilder tests ──────────────────────────────────────────────

    #[test]
    fn builder_user_message() {
        let msg = ChatMessageBuilder::user("hello world").build();
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content(), "hello world");
    }

    #[test]
    fn builder_assistant_message() {
        let msg = ChatMessageBuilder::assistant("thinking...").build();
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content(), "thinking...");
    }

    #[test]
    fn builder_system_message() {
        let msg = ChatMessageBuilder::system("you are helpful").build();
        assert_eq!(msg.role, Role::System);
        assert_eq!(msg.content(), "you are helpful");
    }

    #[test]
    fn builder_thought_message() {
        let msg = ChatMessageBuilder::thought("I should use bash").build();
        assert_eq!(msg.role, Role::Thought);
        assert_eq!(msg.content(), "I should use bash");
    }

    #[test]
    fn builder_appends_text_to_existing_part() {
        let msg = ChatMessageBuilder::assistant("part 1")
            .text(" part 2")
            .build();
        assert_eq!(msg.content(), "part 1 part 2");
        assert_eq!(msg.parts.len(), 1);
    }

    #[test]
    fn builder_adds_reasoning() {
        let msg = ChatMessageBuilder::assistant("answer")
            .reasoning("thinking step by step")
            .build();
        assert_eq!(msg.content(), "answer");
        assert!(msg.parts.iter().any(|p| matches!(p, Part::Reasoning { .. })));
    }

    #[test]
    fn builder_adds_tool_call() {
        let msg = ChatMessageBuilder::assistant("calling tool")
            .tool_call("call_1", "bash", serde_json::json!({"cmd": "ls"}))
            .build();
        assert_eq!(msg.content(), "calling tool");
        assert!(msg
            .parts
            .iter()
            .any(|p| matches!(p, Part::ToolCall { name: n, .. } if n == "bash")));
    }

    #[test]
    fn builder_with_id_and_timestamp() {
        let msg = ChatMessageBuilder::user("hi")
            .id("msg-123")
            .timestamp(1234567890.0)
            .build();
        assert_eq!(msg.id, "msg-123");
        assert_eq!(msg.timestamp, 1234567890.0);
    }

    #[test]
    fn builder_with_tool_call_id() {
        let msg = ChatMessageBuilder::tool("result output")
            .tool_call_id("call_abc")
            .build();
        assert_eq!(msg.tool_call_id, Some("call_abc".to_owned()));
    }

    #[test]
    fn builder_empty_text_is_ignored() {
        let msg = ChatMessageBuilder::assistant("hello").text("").build();
        assert_eq!(msg.content(), "hello");
        assert_eq!(msg.parts.len(), 1);
    }

    #[test]
    fn builder_metadata_pinned() {
        let msg = ChatMessageBuilder::user("important").pinned().build();
        assert!(msg.metadata.pinned);
        assert!(!msg.metadata.hidden_from_user);
    }

    #[test]
    fn builder_metadata_hidden_from_user() {
        let msg = ChatMessageBuilder::assistant("hidden").hidden_from_user().build();
        assert!(msg.metadata.hidden_from_user);
    }

    #[test]
    fn builder_metadata_ephemeral() {
        let msg = ChatMessageBuilder::assistant("temp").ephemeral().build();
        assert!(msg.metadata.ephemeral);
    }

    // ── validate_message tests ─────────────────────────────────────────────────

    #[test]
    fn validate_message_user_is_valid() {
        let msg = ChatMessageBuilder::user("hi").build();
        assert!(validate_message(&msg).is_none());
    }

    #[test]
    fn validate_message_assistant_with_text_is_valid() {
        let msg = ChatMessageBuilder::assistant("hello").build();
        assert!(validate_message(&msg).is_none());
    }

    #[test]
    fn validate_message_assistant_with_tool_calls_is_valid() {
        let msg = ChatMessageBuilder::new(Role::Assistant)
            .tool_call("c1", "bash", serde_json::json!({}))
            .build();
        assert!(validate_message(&msg).is_none());
    }

    #[test]
    fn validate_message_empty_assistant_is_invalid() {
        let msg = ChatMessageBuilder::new(Role::Assistant).build();
        assert!(validate_message(&msg).is_some());
    }

    #[test]
    fn validate_message_tool_without_call_id_is_invalid() {
        let msg = ChatMessageBuilder::tool("result").build();
        assert!(validate_message(&msg).is_some());
    }

    #[test]
    fn validate_message_tool_with_call_id_is_valid() {
        let msg = ChatMessageBuilder::tool("result")
            .tool_call_id("call_1")
            .build();
        assert!(validate_message(&msg).is_none());
    }

    #[test]
    fn validate_message_thought_without_content_is_invalid() {
        let msg = ChatMessageBuilder::thought("").build();
        assert!(validate_message(&msg).is_some());
    }

    // ── validate_messages sequence tests ───────────────────────────────────────

    #[test]
    fn validate_messages_empty_is_valid() {
        let msgs: Vec<ChatMessage> = vec![];
        assert!(validate_messages(&msgs).is_empty());
    }

    #[test]
    fn validate_messages_user_assistant_is_valid() {
        let msgs = vec![
            ChatMessageBuilder::user("hi").build(),
            ChatMessageBuilder::assistant("hello").build(),
        ];
        assert!(validate_messages(&msgs).is_empty());
    }

    // ── MessageOrigin tests ───────────────────────────────────────────────────

    #[test]
    fn message_origin_defaults_to_user() {
        let msg = ChatMessageBuilder::user("hello").build();
        assert_eq!(msg.metadata.origin, MessageOrigin::User);
    }

    #[test]
    fn message_builder_can_set_origin() {
        let msg = ChatMessageBuilder::user("hello")
            .origin(MessageOrigin::Tool)
            .build();
        assert_eq!(msg.metadata.origin, MessageOrigin::Tool);
    }

    #[test]
    fn message_origin_can_be_compaction() {
        let msg = ChatMessageBuilder::assistant("summary")
            .origin(MessageOrigin::Compaction)
            .build();
        assert_eq!(msg.metadata.origin, MessageOrigin::Compaction);
    }

    #[test]
    fn message_origin_can_be_follow_up() {
        let msg = ChatMessageBuilder::user("follow up question")
            .origin(MessageOrigin::FollowUp)
            .build();
        assert_eq!(msg.metadata.origin, MessageOrigin::FollowUp);
    }
}
