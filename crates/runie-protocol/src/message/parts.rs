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
    /// A tool invocation requested by the assistant.
    ToolCall {
        id: String,
        name: String,
        args: Value,
    },
    /// The result returned for a tool invocation.
    ToolResult { id: String, output: String },
}

impl Part {
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text { content: content.into() }
    }

    pub fn reasoning(content: impl Into<String>) -> Self {
        Self::Reasoning { content: content.into() }
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
}
