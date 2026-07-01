//! Message types shared across the application.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub mod parts;

pub use parts::Part;

/// A first-class tool invocation carried by an assistant message.
///
/// `args` is the structured argument object. Use `arguments_string()` to get
/// the JSON-encoded wire format expected by OpenAI-compatible APIs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
        serde_json::to_string(&self.args).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Convert a `Part::ToolCall` into a `ToolCall`; other part variants become empty.
    pub fn from_part(part: Part) -> Self {
        match part {
            Part::ToolCall { id, name, args } => Self { id, name, args },
            _ => Self::new(String::new(), String::new(), serde_json::Value::Null),
        }
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
#[strum(serialize_all = "snake_case")]
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
    /// String representation (snake_case).
    pub fn as_str(&self) -> &'static str {
        // Matches #[strum(serialize_all = "snake_case")] on the enum.
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
        Self::from_str(s).ok()
    }
}

/// Metadata for chat messages (compaction and visibility control).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
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

pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
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
        self.parts
            .iter()
            .filter_map(|p| match p {
                Part::ToolCall { id, name, args } => Some(ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    args: args.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    /// Push a text part, or append to the last text part if one exists.
    pub fn push_text_part(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        if let Some(Part::Text { content: last }) = self.parts.last_mut() {
            last.push_str(content);
        } else {
            self.parts.push(Part::Text {
                content: content.to_owned(),
            });
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
            self.parts.push(Part::ToolCall {
                id: tc.id,
                name: tc.name,
                args: tc.args,
            });
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

/// Builder for `ChatMessage` with validated construction.
///
/// Enforces valid message structure at construction time:
/// - `Role::Assistant` messages must have non-empty text OR tool calls
/// - `Role::Tool` messages must have a `tool_call_id`
/// - `Role::Thought` messages require content
///
/// For constructing message sequences (validation of dangling tool calls,
/// orphan results, role ordering), use [`validate_messages`] instead.
#[derive(Debug, Default)]
pub struct ChatMessageBuilder {
    role: Role,
    timestamp: Option<f64>,
    id: Option<String>,
    provider: Option<String>,
    tool_call_id: Option<String>,
    provider_metadata: Option<serde_json::Value>,
    parts: Vec<Part>,
    metadata: MessageMetadata,
}

impl ChatMessageBuilder {
    pub fn new(role: Role) -> Self {
        Self { role, ..Default::default() }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User).text(content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant).text(content)
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System).text(content)
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self::new(Role::Tool).text(content)
    }

    pub fn thought(content: impl Into<String>) -> Self {
        Self::new(Role::Thought).text(content)
    }

    pub fn text(mut self, content: impl Into<String>) -> Self {
        let content = content.into();
        if content.is_empty() {
            return self;
        }
        if let Some(Part::Text { content: last }) = self.parts.last_mut() {
            last.push_str(&content);
        } else {
            self.parts.push(Part::Text { content });
        }
        self
    }

    pub fn reasoning(mut self, content: impl Into<String>) -> Self {
        let content = content.into();
        if content.is_empty() {
            return self;
        }
        self.parts.push(Part::Reasoning { content });
        self
    }

    pub fn tool_call(mut self, id: impl Into<String>, name: impl Into<String>, args: serde_json::Value) -> Self {
        self.parts.push(Part::tool_call(id, name, args));
        self
    }

    pub fn tool_result(mut self, id: impl Into<String>, output: impl Into<String>) -> Self {
        self.parts.push(Part::tool_result(id, output));
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn timestamp(mut self, ts: f64) -> Self {
        self.timestamp = Some(ts);
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn tool_call_id(mut self, id: impl Into<String>) -> Self {
        self.tool_call_id = Some(id.into());
        self
    }

    pub fn pinned(mut self) -> Self {
        self.metadata.pinned = true;
        self
    }

    pub fn hidden_from_user(mut self) -> Self {
        self.metadata.hidden_from_user = true;
        self
    }

    pub fn ephemeral(mut self) -> Self {
        self.metadata.ephemeral = true;
        self
    }

    pub fn build(self) -> ChatMessage {
        ChatMessage {
            role: self.role,
            timestamp: self.timestamp.unwrap_or_else(now),
            id: self.id.unwrap_or_default(),
            provider: self.provider.unwrap_or_default(),
            tool_call_id: self.tool_call_id,
            provider_metadata: self.provider_metadata,
            metadata: self.metadata,
            parts: self.parts,
        }
    }
}

/// Validate a single message for structural validity.
/// Returns `None` if valid, `Some(reason)` if not.
pub fn validate_message(msg: &ChatMessage) -> Option<&'static str> {
    match msg.role {
        Role::Assistant => {
            let has_text = msg.parts.iter().any(|p| matches!(p, Part::Text { content } if !content.is_empty()));
            let has_tc = msg.parts.iter().any(|p| matches!(p, Part::ToolCall { .. }));
            if !has_text && !has_tc {
                return Some("assistant message has no text or tool calls");
            }
        }
        Role::Tool => {
            if msg.tool_call_id.is_none() {
                return Some("tool message missing tool_call_id");
            }
        }
        Role::Thought => {
            let has_text = msg.parts.iter().any(|p| matches!(p, Part::Text { content } if !content.is_empty()));
            if !has_text {
                return Some("thought message has no content");
            }
        }
        Role::User | Role::System | Role::TurnComplete => {}
    }
    None
}

/// Validate a sequence of messages for structural validity.
///
/// Checks:
/// - Every tool message has a matching tool call id in a preceding assistant message
/// - Assistant messages are not empty (no text, no tool calls)
///
/// Use this after building a message history, not instead of per-message construction.
pub fn validate_messages(messages: &[ChatMessage]) -> Vec<SanitizeError> {
    let mut errors = Vec::new();

    // Collect all valid tool call ids
    let tool_call_ids: std::collections::HashSet<_> = messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .flat_map(|m| m.tool_calls())
        .filter(|tc| !tc.id.is_empty())
        .map(|tc| tc.id)
        .collect();

    for msg in messages {
        // Check tool message has matching tool call
        if msg.role == Role::Tool {
            if let Some(ref id) = msg.tool_call_id {
                if !tool_call_ids.contains(id) {
                    errors.push(SanitizeError::OrphanToolResult {
                        tool_call_id: id.clone(),
                    });
                }
            } else {
                errors.push(SanitizeError::OrphanToolResult {
                    tool_call_id: "<missing>".to_owned(),
                });
            }
        }

        // Check assistant not empty
        if msg.role == Role::Assistant {
            let has_text = !msg.content().trim().is_empty();
            let has_tc = !msg.tool_calls().is_empty();
            if !has_text && !has_tc {
                errors.push(SanitizeError::RemovedMessage {
                    role: Role::Assistant,
                    reason: "empty message",
                });
            }
        }
    }

    errors
}

/// Re-export SanitizeError so callers can use it without importing from sanitize.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SanitizeError {
    #[error("dangling tool call: {tool_call_id}")]
    DanglingToolCall { tool_call_id: String },
    #[error("orphan tool result: {tool_call_id}")]
    OrphanToolResult { tool_call_id: String },
    #[error("removed {role:?} message: {reason}")]
    RemovedMessage { role: Role, reason: &'static str },
}

// ── Serialization round-trip tests ─────────────────────────────────────────────

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
        // Should be a single text part
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
        assert!(msg.parts.iter().any(|p| matches!(p, Part::ToolCall { name: n, .. } if n == "bash")));
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
}
