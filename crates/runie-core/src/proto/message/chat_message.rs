//! Chat message types.

use std::sync::Arc;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use super::metadata::MessageMetadata;
use super::parts::Part;
use super::role::Role;
use super::tool_call::ToolCall;
use crate::hidden_params::{AsHiddenParams, HiddenParams};

/// Current Unix timestamp in seconds.
pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// A chat message in a conversation.
///
/// Built with [`ChatMessageBuilder`]:
///
/// ```
/// use runie_core::proto::message::{ChatMessage, ChatMessageBuilder, Role};
/// let msg = ChatMessageBuilder::user("hello").build();
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Builder)]
#[builder(build_fn(skip))]
pub struct ChatMessage {
    #[builder(setter(into, name = "set_role"))]
    pub role: Role,
    #[builder(default = "now()", setter(into, name = "set_timestamp"))]
    #[serde(default)]
    pub timestamp: f64,
    #[builder(default, setter(into, name = "set_id"))]
    #[serde(default)]
    pub id: String,
    #[builder(default = "String::new()", setter(into, name = "set_provider"))]
    #[serde(default)]
    pub provider: String,
    #[builder(default, setter(into))]
    #[serde(default)]
    pub metadata: MessageMetadata,
    #[builder(default, setter(into, strip_option, name = "set_tool_call_id"))]
    pub tool_call_id: Option<String>,
    #[builder(default, setter(strip_option, name = "set_provider_metadata"))]
    pub provider_metadata: Option<serde_json::Value>,
    #[builder(default, setter(name = "set_parts"))]
    #[serde(default)]
    pub parts: Vec<Part>,
    /// Hidden parameters attached to this message (not serialized).
    #[builder(default, setter(name = "set_hidden_params"))]
    #[serde(skip, default)]
    pub hidden_params: Option<Arc<HiddenParams>>,
}

impl ChatMessage {
    /// Create a new message with optional text content.
    ///
    /// An empty string creates a message with no parts; otherwise creates a
    /// single `Part::Text` part.
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
            hidden_params: None,
        }
    }

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
                Part::ToolCall { id, name, args } => {
                    Some(ToolCall { id: id.clone(), name: name.clone(), args: args.clone() })
                }
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
            self.parts.push(Part::Text { content: content.to_owned() });
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
            self.parts
                .push(Part::ToolCall { id: tc.id, name: tc.name, args: tc.args });
        }
        self
    }

    pub fn with_parts(mut self, parts: Vec<Part>) -> Self {
        self.parts = parts;
        self
    }

    /// Set hidden parameters for internal use (cost, API base, etc.).
    pub fn set_hidden_params(&mut self, params: Arc<HiddenParams>) {
        self.hidden_params = Some(params);
    }
}

impl AsHiddenParams for ChatMessage {
    fn hidden_params(&self) -> Option<&Arc<HiddenParams>> {
        self.hidden_params.as_ref()
    }

    fn hidden_params_mut(&mut self) -> Option<&mut Arc<HiddenParams>> {
        self.hidden_params.as_mut()
    }
}

/// Builder for `ChatMessage` with helper constructors and typed setters.
///
/// Enforces valid message structure at construction time:
/// - `Role::Assistant` messages must have non-empty text OR tool calls
/// - `Role::Tool` messages must have a `tool_call_id`
/// - `Role::Thought` messages require content
///
/// For constructing message sequences (validation of dangling tool calls,
/// orphan results, role ordering), use [`super::validation::validate_messages`] instead.
impl ChatMessageBuilder {
    pub fn new(role: Role) -> Self {
        let mut builder = Self::default();
        builder.set_role(role);
        builder
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

    /// Append text content, or merge with the last text part if one exists.
    pub fn text(mut self, content: impl Into<String>) -> Self {
        let content = content.into();
        if content.is_empty() {
            return self;
        }
        let parts = self.parts.get_or_insert_with(Vec::new);
        if let Some(Part::Text { content: last }) = parts.last_mut() {
            last.push_str(&content);
        } else {
            parts.push(Part::Text { content });
        }
        self
    }

    pub fn reasoning(mut self, content: impl Into<String>) -> Self {
        let content = content.into();
        if content.is_empty() {
            return self;
        }
        let parts = self.parts.get_or_insert_with(Vec::new);
        parts.push(Part::Reasoning { content });
        self
    }

    pub fn tool_call(mut self, id: impl Into<String>, name: impl Into<String>, args: serde_json::Value) -> Self {
        let parts = self.parts.get_or_insert_with(Vec::new);
        parts.push(Part::tool_call(id, name, args));
        self
    }

    pub fn tool_result(mut self, id: impl Into<String>, output: impl Into<String>) -> Self {
        let parts = self.parts.get_or_insert_with(Vec::new);
        parts.push(Part::tool_result(id, output));
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

    pub fn tool_call_id(mut self, id: impl Into<String>) -> Self {
        self.tool_call_id = Some(Some(id.into()));
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn hidden_from_user(mut self) -> Self {
        self.metadata
            .get_or_insert_with(Default::default)
            .hidden_from_user = true;
        self
    }

    pub fn pinned(mut self) -> Self {
        self.metadata.get_or_insert_with(Default::default).pinned = true;
        self
    }

    pub fn ephemeral(mut self) -> Self {
        self.metadata.get_or_insert_with(Default::default).ephemeral = true;
        self
    }

    pub fn origin(mut self, origin: super::role::MessageOrigin) -> Self {
        self.metadata.get_or_insert_with(Default::default).origin = origin;
        self
    }

    pub fn build(self) -> ChatMessage {
        let role = self.role.unwrap_or(Role::User);
        let timestamp = self.timestamp.unwrap_or_else(now);
        let id = self.id.unwrap_or_default();
        let provider = self.provider.unwrap_or_default();
        let metadata = self.metadata.unwrap_or_default();
        let parts = self.parts.unwrap_or_default();
        ChatMessage {
            role,
            timestamp,
            id,
            provider,
            metadata,
            tool_call_id: self.tool_call_id.flatten(),
            provider_metadata: self.provider_metadata.flatten(),
            parts,
            hidden_params: self.hidden_params.unwrap_or_default(),
        }
    }

    /// Attach hidden parameters to this message.
    pub fn with_hidden_params(mut self, params: Arc<HiddenParams>) -> Self {
        self.set_hidden_params(Some(params));
        self
    }
}
