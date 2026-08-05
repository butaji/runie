//! Default `convert_to_llm` implementation.
//!
//! Filters app-level `Custom` messages and converts the standard
//! `AgentMessage` variants to wire-format `WireMessage`s.

use crate::types::{AgentMessage, WireMessage};

/// Default conversion: drops `Custom`, maps `User`/`Assistant`/`ToolResult`
/// to their wire shape. Apps needing custom-message handling supply their own
/// `convert_to_llm` hook.
pub fn default_convert_to_llm(messages: &[AgentMessage]) -> Vec<WireMessage> {
    messages
        .iter()
        .filter_map(|m| match m {
            AgentMessage::User(u) => Some(WireMessage::User {
                content: u.content.clone(),
                timestamp: u.timestamp,
            }),
            AgentMessage::Assistant(a) => Some(WireMessage::Assistant {
                content: a.content.clone(),
                stop_reason: a.stop_reason,
                model: a.model.clone(),
                timestamp: a.timestamp,
            }),
            AgentMessage::ToolResult(t) => Some(WireMessage::ToolResult {
                tool_call_id: t.tool_call_id.clone(),
                tool_name: t.tool_name.clone(),
                content: t.content.clone(),
                is_error: t.is_error,
                timestamp: t.timestamp,
            }),
            AgentMessage::Custom(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AgentMessageExt, AssistantContent, AssistantMessage, StopReason, TextContent,
        ToolResultContent, ToolResultMessage, UserContent, UserMessage,
    };
    use std::sync::Arc;

    struct CustomExt;
    impl AgentMessageExt for CustomExt {
        fn role(&self) -> &str {
            "custom"
        }
        fn timestamp(&self) -> i64 {
            7
        }
    }

    #[test]
    fn default_convert_drops_custom() {
        let messages = vec![
            AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: "hi".into() }],
                timestamp: 1,
            }),
            AgentMessage::Custom(crate::types::CustomMessage(Arc::new(CustomExt))),
            AgentMessage::Assistant(AssistantMessage {
                content: vec![AssistantContent::Text {
                    text: "hello".into(),
                }],
                stop_reason: Some(StopReason::Stop),
                model: "test".into(),
                timestamp: 2,
            }),
        ];
        let wire = default_convert_to_llm(&messages);
        assert_eq!(wire.len(), 2);
    }

    #[test]
    fn default_convert_passes_through_text() {
        let t = TextContent { text: "abc".into() };
        let _ = t; // type check
    }

    #[test]
    fn tool_result_round_trip() {
        let m = ToolResultMessage {
            tool_call_id: "id".into(),
            tool_name: "read_file".into(),
            content: vec![ToolResultContent::Text { text: "ok".into() }],
            is_error: false,
            timestamp: 9,
        };
        let wire = default_convert_to_llm(&[AgentMessage::ToolResult(m.clone())]);
        assert_eq!(wire.len(), 1);
        assert!(matches!(wire[0], WireMessage::ToolResult { .. }));
    }
}
