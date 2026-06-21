//! Message types shared across the application.

pub mod parts;
pub use parts::Part;

pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// A first-class tool invocation carried by an assistant message.
///
/// `arguments` is the JSON-encoded argument object, matching the OpenAI
/// `function.arguments` wire format. Providers that need structured input
/// can deserialize it; providers that use plain-text tool markers can ignore
/// it and rely on `ChatMessage.content` instead.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: String) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Role {
    #[default]
    User,
    Thought,
    Assistant,
    Tool,
    TurnComplete,
    System,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Thought => "thought",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
            Role::TurnComplete => "turn_complete",
            Role::System => "system",
        }
    }

    /// Convert from API string representation.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Role::User),
            "thought" => Some(Role::Thought),
            "assistant" => Some(Role::Assistant),
            "tool" => Some(Role::Tool),
            "turn_complete" => Some(Role::TurnComplete),
            "system" => Some(Role::System),
            _ => None,
        }
    }
}

impl ChatMessage {
    /// Convert to a provider-agnostic message (drops metadata).
    pub fn to_provider_message(&self) -> crate::provider::Message {
        match self.role {
            Role::System => crate::provider::Message::System {
                content: self.content.clone(),
            },
            Role::User | Role::Thought => crate::provider::Message::User {
                content: self.content.clone(),
            },
            Role::Assistant => crate::provider::Message::Assistant {
                content: self.content.clone(),
                tool_calls: self.tool_calls.clone(),
            },
            Role::Tool | Role::TurnComplete => crate::provider::Message::ToolResult {
                content: self.content.clone(),
                tool_call_id: self.tool_call_id.clone(),
            },
        }
    }
}

/// Metadata for chat messages (compaction and visibility control).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MessageMetadata {
    /// Message is pinned and won't be compacted.
    #[serde(default)]
    pub pinned: bool,
    /// Message is hidden from user display but still sent to the model.
    #[serde(default)]
    pub hidden_from_user: bool,
    /// Message is omitted from persistence (ephemeral).
    #[serde(default)]
    pub ephemeral: bool,
    /// This message is a compaction summary (replaces older messages).
    #[serde(default)]
    pub compacted: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    pub timestamp: f64,
    pub id: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub metadata: MessageMetadata,
    /// Tool calls issued by the assistant in this message.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// For `Role::Tool` messages, the id of the assistant tool call this
    /// result answers. Required by OpenAI-compatible APIs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Provider-specific round-trip state (signatures, reasoning format, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<serde_json::Value>,
    /// Typed parts of this message (text, reasoning, tool calls, results).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    pub fn tool_result(content: impl Into<String>) -> Self {
        Self::new(Role::Tool, content)
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self::new(Role::Tool, content)
    }

    pub fn with_tool_call_id(mut self, id: impl Into<String>) -> Self {
        self.tool_call_id = Some(id.into());
        self
    }

    pub fn with_tool_calls(mut self, calls: Vec<ToolCall>) -> Self {
        self.tool_calls = calls;
        self
    }

    pub fn with_parts(mut self, parts: Vec<Part>) -> Self {
        self.parts = parts;
        self
    }

    fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            timestamp: now(),
            id: String::new(),
            provider: String::new(),
            metadata: MessageMetadata::default(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            provider_metadata: None,
            parts: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Message;

    #[test]
    fn role_as_str_matches_provider_expectations() {
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Assistant.as_str(), "assistant");
        assert_eq!(Role::System.as_str(), "system");
        assert_eq!(Role::Tool.as_str(), "tool");
    }

    #[test]
    fn chat_message_round_trip_json() {
        let msg = ChatMessage {
            role: Role::User,
            content: "hello".to_string(),
            timestamp: 1234567890.0,
            id: "msg-1".to_string(),
            provider: "openai".to_string(),
            metadata: MessageMetadata::default(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            provider_metadata: None,
            parts: Vec::new(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, Role::User);
        assert_eq!(parsed.content, "hello");
        assert_eq!(parsed.id, "msg-1");
    }

    #[test]
    fn chat_message_to_provider_message() {
        let msg = ChatMessage {
            role: Role::User,
            content: "hello".to_string(),
            timestamp: 0.0,
            id: "1".to_string(),
            provider: String::new(),
            metadata: MessageMetadata::default(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            provider_metadata: None,
            parts: Vec::new(),
        };
        let provider_msg = msg.to_provider_message();
        assert!(matches!(provider_msg, Message::User { content } if content == "hello"));
    }

    #[test]
    fn chat_message_to_provider_message_with_tool_call() {
        let msg = ChatMessage {
            role: Role::Assistant,
            content: "".to_string(),
            timestamp: 0.0,
            id: "2".to_string(),
            provider: String::new(),
            metadata: MessageMetadata::default(),
            tool_calls: vec![ToolCall::new(
                "call_1",
                "read_file",
                r#"{"path":"Cargo.toml"}"#.to_string(),
            )],
            tool_call_id: None,
            provider_metadata: None,
            parts: Vec::new(),
        };
        let provider_msg = msg.to_provider_message();
        match provider_msg {
            Message::Assistant {
                content,
                tool_calls,
            } => {
                assert!(content.is_empty());
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].id, "call_1");
                assert_eq!(tool_calls[0].name, "read_file");
            }
            other => panic!("expected Assistant message, got {:?}", other),
        }
    }

    #[test]
    fn chat_message_to_provider_message_with_tool_result_id() {
        let msg = ChatMessage {
            role: Role::Tool,
            content: "file contents".to_string(),
            timestamp: 0.0,
            id: "3".to_string(),
            provider: String::new(),
            metadata: MessageMetadata::default(),
            tool_calls: Vec::new(),
            tool_call_id: Some("call_1".to_string()),
            provider_metadata: None,
            parts: Vec::new(),
        };
        let provider_msg = msg.to_provider_message();
        match provider_msg {
            Message::ToolResult {
                content,
                tool_call_id,
            } => {
                assert_eq!(content, "file contents");
                assert_eq!(tool_call_id, Some("call_1".to_string()));
            }
            other => panic!("expected ToolResult message, got {:?}", other),
        }
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
    fn chat_message_with_parts_round_trips_json() {
        use crate::message::Part;
        let msg = ChatMessage {
            role: Role::Assistant,
            content: String::new(),
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
        let json = r#"{"role":"Assistant","content":"hi","timestamp":1.0,"id":"a1"}"#;
        let parsed: ChatMessage = serde_json::from_str(json).unwrap();
        assert!(parsed.parts.is_empty());
        assert_eq!(parsed.content, "hi");
    }
}
