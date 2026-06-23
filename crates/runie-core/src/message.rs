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
/// `args` is the structured argument object. Use `arguments_string()` to get
/// the JSON-encoded wire format expected by OpenAI-compatible APIs.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Structured arguments. Serializes to JSON string for OpenAI wire format.
    #[serde(default)]
    pub args: serde_json::Value,
}

impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            args,
        }
    }

    /// Construct a ToolCall from a JSON string for the arguments field.
    pub fn with_json_args(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl AsRef<str>,
    ) -> Self {
        let args: serde_json::Value =
            serde_json::from_str(arguments.as_ref()).unwrap_or(serde_json::Value::Null);
        Self::new(id, name, args)
    }

    /// Serialize arguments to a JSON string for the OpenAI wire format.
    pub fn arguments_string(&self) -> String {
        serde_json::to_string(&self.args).unwrap_or_else(|_| "{}".to_string())
    }
}

impl From<crate::message::Part> for ToolCall {
    fn from(part: crate::message::Part) -> Self {
        match part {
            crate::message::Part::ToolCall { id, name, args } => ToolCall { id, name, args },
            _ => ToolCall::new(String::new(), String::new(), serde_json::Value::Null),
        }
    }
}

impl From<crate::tool_parser::ParsedToolCall> for ToolCall {
    fn from(call: crate::tool_parser::ParsedToolCall) -> Self {
        ToolCall {
            id: call.id.unwrap_or_default(),
            name: call.name,
            args: call.args,
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
    /// Returns the concatenated text content from all `Part::Text` variants.
    pub fn content(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                Part::Text { content } => Some(content.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Returns tool calls extracted from `Part::ToolCall` variants.
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.parts.iter().filter_map(|p| match p {
            Part::ToolCall { id, name, args } => Some(ToolCall { id: id.clone(), name: name.clone(), args: args.clone() }),
            _ => None,
        }).collect()
    }

    /// Push a text part, or append to the last text part if one exists.
    pub fn push_text_part(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        if let Some(Part::Text { content: last }) = self.parts.last_mut() {
            last.push_str(content);
        } else {
            self.parts.push(Part::Text { content: content.to_string() });
        }
    }

    /// Set the last text part's content (or push a new text part).
    pub fn set_text_part(&mut self, content: String) {
        if let Some(Part::Text { content: last }) = self.parts.last_mut() {
            *last = content;
        } else {
            self.parts.push(Part::Text { content });
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
    pub timestamp: f64,
    pub id: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub metadata: MessageMetadata,
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

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub fn with_tool_call_id(mut self, id: impl Into<String>) -> Self {
        self.tool_call_id = Some(id.into());
        self
    }

    pub fn with_tool_calls(mut self, calls: Vec<ToolCall>) -> Self {
        for tc in calls {
            self.parts.push(Part::ToolCall { id: tc.id, name: tc.name, args: tc.args });
        }
        self
    }

    pub fn with_parts(mut self, parts: Vec<Part>) -> Self {
        self.parts = parts;
        self
    }

    pub fn new(role: Role, content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            role,
            timestamp: now(),
            id: String::new(),
            provider: String::new(),
            metadata: MessageMetadata::default(),
            tool_call_id: None,
            provider_metadata: None,
            parts: if content.is_empty() {
                Vec::new()
            } else {
                vec![Part::Text { content }]
            },
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
            parts: vec![Part::Text { content: "hello".into() }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, Role::User);
        assert_eq!(parsed.content(), "hello");
        assert_eq!(parsed.id, "msg-1");
    }

    #[test]
    fn chat_message_to_provider_message() {
        let msg = ChatMessage::new(Role::User, "hello");
        let provider_msg = msg.to_provider_message();
        assert!(matches!(provider_msg, Message::User { content } if content == "hello"));
    }

    #[test]
    fn chat_message_to_provider_message_with_tool_call() {
        let msg = ChatMessage {
            role: Role::Assistant,
            timestamp: 0.0,
            id: "2".to_string(),
            provider: String::new(),
            metadata: MessageMetadata::default(),
            tool_call_id: None,
            provider_metadata: None,
            parts: vec![
                Part::Text { content: String::new() },
                Part::ToolCall {
                    id: "call_1".into(),
                    name: "read_file".into(),
                    args: serde_json::json!({"path": "Cargo.toml"}),
                },
            ],
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
            timestamp: 0.0,
            id: "3".to_string(),
            provider: String::new(),
            metadata: MessageMetadata::default(),
            tool_call_id: Some("call_1".to_string()),
            provider_metadata: None,
            parts: vec![Part::Text { content: "file contents".into() }],
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
        // Old format with content field deserializes to empty parts
        let json = r#"{"role":"Assistant","content":"hi","timestamp":1.0,"id":"a1"}"#;
        let parsed: ChatMessage = serde_json::from_str(json).unwrap();
        // Legacy format has no parts - content getter returns empty for backward compat
        assert!(parsed.parts.is_empty());
        assert_eq!(parsed.content(), "");
    }

    #[test]
    fn to_provider_message_serializes_from_parts() {
        let msg = ChatMessage {
            role: Role::Assistant,
            parts: vec![
                Part::Text { content: "hi".into() },
                Part::ToolCall { id: "c1".into(), name: "bash".into(), args: serde_json::json!({"cmd": "ls"}) },
            ],
            ..Default::default()
        };
        let provider_msg = msg.to_provider_message();
        match provider_msg {
            Message::Assistant { content, tool_calls } => {
                assert_eq!(content, "hi");
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].name, "bash");
            }
            other => panic!("expected Assistant message, got {:?}", other),
        }
    }
}
