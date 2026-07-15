//! Typed parts of an assistant or tool message.
//!
//! A `Part` represents a semantically distinct block inside a message: plain
//! text, reasoning/thinking, a tool call, or a tool result. Persisting messages
//! as a list of parts makes streaming replay, tool-call grouping, and
//! multimodal content easier than storing a single monolithic string.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A typed block inside a `ChatMessage`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    /// Plain assistant text.
    Text { content: String },
    /// Model reasoning / thinking block.
    Reasoning { content: String },
    /// Redacted/encrypted thinking (from models like DeepSeek with signature)
    ReasoningEncrypted { data: String, signature: Option<String> },
    /// Anthropic-style thinking block with explicit type and signature
    AnthropicThinking {
        content: String,
        signature: Option<String>,
    },
    /// A tool invocation requested by the assistant.
    ToolCall {
        id: String,
        name: String,
        args: Value,
    },
    /// The result returned for a tool invocation.
    ToolResult { id: String, output: String },
    /// Tool call requiring user confirmation
    ToolConfirmation {
        id: String,
        name: String,
        args: Value,
        description: Option<String>,
    },
    /// Inline image (base64 encoded)
    Image {
        data: String,
        mime_type: String,
    },
    /// Structured data / JSON
    Data { data: String, format: Option<String> },
    /// Web search invocation
    WebSearch { query: String },
    /// Diff/changelist output
    Diff { content: String, diff_type: String },
    /// ANSI-styled content
    Ansi { raw: String, plain: String },
}

impl Part {
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            content: content.into(),
        }
    }

    pub fn reasoning(content: impl Into<String>) -> Self {
        Self::Reasoning {
            content: content.into(),
        }
    }

    pub fn reasoning_encrypted(data: impl Into<String>, signature: Option<String>) -> Self {
        Self::ReasoningEncrypted {
            data: data.into(),
            signature,
        }
    }

    pub fn anthropic_thinking(content: impl Into<String>, signature: Option<String>) -> Self {
        Self::AnthropicThinking {
            content: content.into(),
            signature,
        }
    }

    pub fn tool_call(id: impl Into<String>, name: impl Into<String>, args: Value) -> Self {
        Self::ToolCall {
            id: id.into(),
            name: name.into(),
            args,
        }
    }

    pub fn tool_result(id: impl Into<String>, output: impl Into<String>) -> Self {
        Self::ToolResult {
            id: id.into(),
            output: output.into(),
        }
    }

    pub fn tool_confirmation(
        id: impl Into<String>,
        name: impl Into<String>,
        args: Value,
        description: Option<String>,
    ) -> Self {
        Self::ToolConfirmation {
            id: id.into(),
            name: name.into(),
            args,
            description,
        }
    }

    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    pub fn data(data: impl Into<String>, format: Option<String>) -> Self {
        Self::Data { data: data.into(), format }
    }

    pub fn web_search(query: impl Into<String>) -> Self {
        Self::WebSearch {
            query: query.into(),
        }
    }

    pub fn diff(content: impl Into<String>, diff_type: impl Into<String>) -> Self {
        Self::Diff {
            content: content.into(),
            diff_type: diff_type.into(),
        }
    }

    pub fn ansi(raw: impl Into<String>, plain: impl Into<String>) -> Self {
        Self::Ansi {
            raw: raw.into(),
            plain: plain.into(),
        }
    }
}
