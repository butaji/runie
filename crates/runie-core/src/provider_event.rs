//! Canonical provider event vocabulary.
//!
//! Normalizes events from all providers (OpenAI, Anthropic, Ollama, etc.)
//! into a unified vocabulary distinct from the internal `Event` type.

use crate::provider::ProviderError;
use serde::{Deserialize, Serialize};

/// Unified provider event stream type.
///
/// All providers emit the same event vocabulary regardless of their
/// underlying API shape. This is the canonical type for the streaming
/// pipeline; convert to `Event` at the TUI/headless boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum ProviderEvent {
    /// An assistant text block has started.
    TextStart { id: String },
    /// A delta of text content from the assistant.
    TextDelta(String),
    /// An assistant text block has ended.
    TextEnd { id: String },
    /// A delta of thinking/reasoning content (if supported).
    ThinkingDelta(String),
    /// Thinking/reasoning block started (used by ThinkFilter for inline tags).
    ThinkingStart { id: String },
    /// Thinking/reasoning block ended (used by ThinkFilter for inline tags).
    ThinkingEnd { id: String },
    /// An LLM started invoking a tool.
    ToolCallStart { id: String, name: String },
    /// A delta of tool input content.
    ToolCallInputDelta { id: String, delta: String },
    /// An LLM finished a tool invocation.
    ToolCallEnd { id: String },
    /// An error occurred during generation.
    Error(ModelError),
    /// Token usage information.
    Usage {
        input_tokens: usize,
        output_tokens: usize,
    },
    /// Generation finished.
    Finish { reason: StopReason },
}

/// Why the generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Natural end of content.
    Stop,
    /// Hit max tokens limit.
    Length,
    /// Model refused to continue.
    ContentFilter,
    /// Tool calls completed.
    ToolCalls,
    /// Stop sequence encountered.
    StopSequence,
    /// Model error or other.
    Unknown,
}

impl StopReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            StopReason::Stop => "stop",
            StopReason::Length => "length",
            StopReason::ContentFilter => "content_filter",
            StopReason::ToolCalls => "tool_calls",
            StopReason::StopSequence => "stop_sequence",
            StopReason::Unknown => "unknown",
        }
    }
}

/// Model-specific errors (distinct from `ProviderError`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum ModelError {
    /// LLM returned invalid JSON.
    JsonDecode(String),
    /// Token limit exceeded.
    ContextLength { limit: usize, used: usize },
    /// Model refused the request.
    Refusal(String),
    /// Rate limit hit.
    RateLimit { retry_after_secs: Option<u32> },
    /// An underlying error (network, parse, etc.).
    Other(String),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::JsonDecode(msg) => write!(f, "JSON decode error: {msg}"),
            ModelError::ContextLength { limit, used } => {
                write!(f, "Context length exceeded: used {used}, limit {limit}")
            }
            ModelError::Refusal(msg) => write!(f, "Model refused: {msg}"),
            ModelError::RateLimit { retry_after_secs } => {
                if let Some(secs) = retry_after_secs {
                    write!(f, "Rate limited, retry after {secs}s")
                } else {
                    write!(f, "Rate limited")
                }
            }
            ModelError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ModelError {}

impl From<anyhow::Error> for ModelError {
    fn from(e: anyhow::Error) -> Self {
        ModelError::Other(e.to_string())
    }
}

impl From<ProviderError> for ModelError {
    fn from(e: ProviderError) -> Self {
        ModelError::Other(e.to_string())
    }
}

impl From<&str> for ModelError {
    fn from(s: &str) -> Self {
        ModelError::Other(s.to_owned())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_event_serialize_deserialize() {
        let event = ProviderEvent::TextDelta("Hello, world!".into());
        let json = serde_json::to_string(&event).unwrap();
        let parsed: ProviderEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ProviderEvent::TextDelta(t) if t == "Hello, world!"));
    }

    #[test]
    fn provider_event_tool_call_roundtrip() {
        let event = ProviderEvent::ToolCallStart {
            id: "call_abc".into(),
            name: "bash".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: ProviderEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ProviderEvent::ToolCallStart { id, name }
            if id == "call_abc" && name == "bash"));
    }

    #[test]
    fn model_error_from_provider_error() {
        let err = ProviderError::MissingApiKey("openai".into());
        let model_err: ModelError = err.into();
        assert!(matches!(model_err, ModelError::Other(s) if s.contains("openai")));
    }

    #[test]
    fn model_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("network timeout");
        let model_err: ModelError = anyhow_err.into();
        assert!(matches!(model_err, ModelError::Other(s) if s.contains("network timeout")));
    }

    #[test]
    fn stop_reason_serialization() {
        let reason = StopReason::ToolCalls;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, "\"tool_calls\"");
    }
}
