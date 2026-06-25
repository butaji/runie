//! Session snapshot DTO — serializable conversation state used for
//! import/export and `/restore`. This is *not* a persistence backend;
//! runtime save/load use `crate::session::store::SessionStore`.

pub mod index;
pub mod replay;
pub mod store;
pub mod tree;

use crate::model::ChatMessage;
use serde::{Deserialize, Serialize};

/// Session snapshot — serializable conversation state.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Session {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub created_at: f64,
    pub updated_at: f64,
    pub messages: Vec<ChatMessage>,
    pub provider: String,
    pub model: String,
    pub theme_name: String,
    pub thinking_level: crate::model::ThinkingLevel,
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_tree: Option<crate::session::tree::SessionTree>,
}

impl Session {
    /// Build a JSON session snapshot from the current application state.
    pub fn from_state(state: &crate::model::AppState, name: String) -> Self {
        Self {
            name,
            display_name: state.session().session_display_name.clone(),
            created_at: state.session().session_created_at,
            updated_at: crate::model::now(),
            messages: state.session().messages.clone(),
            provider: state.config().current_provider.clone(),
            model: state.config().current_model.clone(),
            theme_name: state.config().theme_name.clone(),
            thinking_level: state.config().thinking_level,
            read_only: state.config().read_only,
            session_tree: state.session().session_tree.clone(),
        }
    }
}

/// Format a slice of chat messages as Markdown for export or sharing.
pub fn format_as_markdown(messages: &[ChatMessage], display_name: Option<&str>) -> String {
    let mut lines = Vec::new();
    let title = display_name.unwrap_or("Session");
    lines.push(format!("# {}\n", title));

    for msg in messages {
        let role_label = match msg.role {
            crate::model::Role::User => "User",
            crate::model::Role::Assistant => "Assistant",
            crate::model::Role::System => "System",
            crate::model::Role::Tool => "Tool",
            crate::model::Role::Thought => "Thought",
            crate::model::Role::TurnComplete => continue,
        };
        lines.push(format!("## {}\n", role_label));
        lines.push(msg.content());
        lines.push(String::new());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatMessage, Part, Role};

    fn sample_session(name: &str) -> Session {
        Session {
            name: name.to_string(),
            display_name: None,
            created_at: 1.0,
            updated_at: 2.0,
            messages: vec![
                ChatMessage {
                    role: Role::User,
                    timestamp: 1.0,
                    id: "req.0".into(),
                    parts: vec![Part::Text { content: "hi".into() }],
                    ..Default::default()
                },
                ChatMessage {
                    role: Role::Assistant,
                    timestamp: 2.0,
                    id: "resp.0".into(),
                    parts: vec![Part::Text { content: "hello".into() }],
                    ..Default::default()
                },
            ],
            provider: "mock".into(),
            model: "echo".into(),
            theme_name: "runie".into(),
            thinking_level: crate::model::ThinkingLevel::Off,
            read_only: false,
            session_tree: None,
        }
    }

    #[test]
    fn serialize_role_roundtrip() {
        let role = Role::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        let decoded: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(role, decoded);
    }

    #[test]
    fn serialize_chat_message_roundtrip() {
        let msg = ChatMessage {
            role: Role::User,
            timestamp: 1.5,
            id: "req.1".into(),
            parts: vec![Part::Text { content: "test".into() }],
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, decoded.role);
        assert_eq!(msg.content(), decoded.content());
        assert_eq!(msg.timestamp, decoded.timestamp);
        assert_eq!(msg.id, decoded.id);
    }

    #[test]
    fn serialize_session_full_roundtrip() {
        let session = sample_session("full");
        let json = serde_json::to_string_pretty(&session).unwrap();
        let decoded: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(session.name, decoded.name);
        assert_eq!(session.messages.len(), decoded.messages.len());
        assert_eq!(session.theme_name, decoded.theme_name);
        assert_eq!(session.thinking_level, decoded.thinking_level);
    }

    #[test]
    fn session_persists_provider() {
        let mut session = sample_session("provider_test");
        session.messages[1].provider = "openai".to_string();
        let json = serde_json::to_string(&session).unwrap();
        let loaded: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.messages[1].provider, "openai");
    }

    #[test]
    fn format_session_markdown_includes_title() {
        let session = sample_session("md_test");
        let md = format_as_markdown(&session.messages, Some("My Session"));
        assert!(md.starts_with("# My Session\n"));
    }

    #[test]
    fn format_session_markdown_includes_roles() {
        let session = sample_session("md_test");
        let md = format_as_markdown(&session.messages, None);
        assert!(md.contains("## User\n"));
        assert!(md.contains("## Assistant\n"));
    }

    #[test]
    fn format_session_markdown_skips_turn_complete() {
        let mut session = sample_session("md_test");
        session.messages.push(ChatMessage {
            role: Role::TurnComplete,
            parts: vec![crate::message::Part::Text { content: String::new() }],
            timestamp: 3.0,
            id: "tc".into(),
            ..Default::default()
        });
        let md = format_as_markdown(&session.messages, None);
        assert!(!md.contains("TurnComplete"));
    }
}
