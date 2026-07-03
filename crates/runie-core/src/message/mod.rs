//! Message types shared across the application.
//!
//! Canonical definitions live in `crate::proto::message`. This module re-exports
//! them for backward compatibility and adds runie-core-specific `From` conversions.
//!
//! ## Validation
//!
//! Use [`validate_messages`] to check a sequence for dangling tool calls and orphan
//! results. Use [`sanitize_messages`] to validate AND trim whitespace.

pub use crate::proto::message::{
    ChatMessage, MessageMetadata, MessageOrigin, Part, Role, ToolCall,
};

pub use crate::proto::message::{validate_message, validate_messages, SanitizeError};

pub use crate::proto::message::now;

/// Validate and trim a message sequence.
///
/// See [`crate::proto::message::validate_messages`] for the underlying validation.
pub use crate::sanitize::sanitize_messages;

impl From<crate::tool::ParsedToolCall> for ToolCall {
    fn from(call: crate::tool::ParsedToolCall) -> Self {
        ToolCall {
            id: call.id.unwrap_or_default(),
            name: call.name,
            args: call.args,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_as_str_matches_provider_expectations() {
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Assistant.as_str(), "assistant");
        assert_eq!(Role::System.as_str(), "system");
        assert_eq!(Role::Tool.as_str(), "tool");
    }

    #[test]
    fn chat_message_content_getter_concatenates_text_parts() {
        let msg = ChatMessage {
            parts: vec![
                Part::Text {
                    content: "a".into(),
                },
                Part::Reasoning {
                    content: "r".into(),
                },
                Part::Text {
                    content: "b".into(),
                },
            ],
            ..Default::default()
        };
        assert_eq!(msg.content(), "ab");
    }

    #[test]
    fn chat_message_tool_calls_getter_extracts_from_parts() {
        let msg = ChatMessage {
            parts: vec![
                Part::Text {
                    content: "hi".into(),
                },
                Part::ToolCall {
                    id: "c1".into(),
                    name: "bash".into(),
                    args: serde_json::json!({}),
                },
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
            parts: vec![Part::ToolCall {
                id: "c1".into(),
                name: "bash".into(),
                args: serde_json::json!({}),
            }],
            ..Default::default()
        };
        assert_eq!(msg.content(), "");
    }

    #[test]
    fn chat_message_round_trip_json() {
        let msg = ChatMessage {
            role: Role::User,
            timestamp: 1234567890.0,
            id: "msg-1".to_string(),
            provider: "openai".to_string(),
            metadata: MessageMetadata::default(),
            tool_call_id: None,
            provider_metadata: None,
            parts: vec![Part::Text {
                content: "hello".into(),
            }],
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
                Part::Text {
                    content: "hello".into(),
                },
                Part::ToolCall {
                    id: "call_1".into(),
                    name: "list_dir".into(),
                    args: serde_json::json!({"path": "."}),
                },
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
    fn provider_view_from_chat_message() {
        let msg = ChatMessage {
            role: Role::Assistant,
            parts: vec![
                Part::Text {
                    content: "hi".into(),
                },
                Part::ToolCall {
                    id: "c1".into(),
                    name: "bash".into(),
                    args: serde_json::json!({"cmd": "ls"}),
                },
            ],
            ..Default::default()
        };
        assert_eq!(msg.content(), "hi");
        let tcs = msg.tool_calls();
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0].name, "bash");
        assert_eq!(tcs[0].id, "c1");
        assert_eq!(tcs[0].args["cmd"], "ls");
    }

    #[test]
    fn role_from_str_round_trip() {
        for role in [Role::User, Role::Assistant, Role::System, Role::Tool] {
            let s = role.as_str();
            let parsed = Role::parse(s).unwrap();
            assert_eq!(parsed, role);
        }
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

    #[test]
    fn parsed_tool_call_maps_to_canonical() {
        use crate::tool::parse::ParsedToolCall;
        let parsed_call = ParsedToolCall {
            name: "read_file".into(),
            args: serde_json::json!({"path": "Cargo.toml"}),
            id: Some("call_123".into()),
        };
        let tc: ToolCall = parsed_call.into();
        assert_eq!(tc.id, "call_123");
        assert_eq!(tc.name, "read_file");
        assert_eq!(tc.args["path"], "Cargo.toml");
    }

    #[test]
    fn chat_message_with_parts_round_trips_json() {
        let msg = ChatMessage {
            role: Role::Assistant,
            timestamp: 1.0,
            id: "a1".into(),
            parts: vec![
                Part::Text {
                    content: "hello".into(),
                },
                Part::ToolCall {
                    id: "call_1".into(),
                    name: "list_dir".into(),
                    args: serde_json::json!({"path": "."}),
                },
            ],
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.parts, msg.parts);
    }

    #[test]
    fn chat_message_without_parts_deserializes_empty_vec() {
        let json = r#"{"role":"Assistant","timestamp":1.0,"id":"a1"}"#;
        let parsed: ChatMessage = serde_json::from_str(json).unwrap();
        assert!(parsed.parts.is_empty());
        assert_eq!(parsed.content(), "");
    }
}
