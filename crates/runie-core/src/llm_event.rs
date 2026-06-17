//! Provider-agnostic LLM event stream types.
//!
//! Normalizes events from all providers (OpenAI, Anthropic, Ollama, etc.)
//! into a unified vocabulary.

use crate::provider::ProviderError;
use serde::{Deserialize, Serialize};

/// Unified LLM event stream type.
///
/// All providers emit the same event vocabulary regardless of their
/// underlying API shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum LLMEvent {
    /// A delta of text content from the assistant.
    TextDelta(String),
    /// A delta of thinking/reasoning content (if supported).
    ThinkingDelta(String),
    /// An LLM started invoking a tool.
    ToolCallStart { id: String, name: String },
    /// A delta of tool input content.
    ToolCallInputDelta { id: String, delta: String },
    /// An LLM finished a tool invocation.
    ToolCallEnd { id: String },
    /// An error occurred during generation.
    Error(LLMError),
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

/// LLM-specific errors (distinct from ProviderError).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum LLMError {
    /// LLM returned invalid JSON.
    JsonDecode(String),
    /// Token limit exceeded.
    ContextLength { limit: usize, used: usize },
    /// Model refused the request.
    Refusal(String),
    /// Rate limit hit.
    RateLimit { retry_after_secs: Option<u32> },
    /// Other model error.
    Other(String),
}

impl From<ProviderError> for LLMError {
    fn from(e: ProviderError) -> Self {
        LLMError::Other(e.to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_event_serialize_deserialize() {
        let event = LLMEvent::TextDelta("Hello, world!".into());
        let json = serde_json::to_string(&event).unwrap();
        let parsed: LLMEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, LLMEvent::TextDelta(t) if t == "Hello, world!"));
    }

    #[test]
    fn llm_event_tool_call_roundtrip() {
        let event = LLMEvent::ToolCallStart {
            id: "call_abc".into(),
            name: "bash".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: LLMEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, LLMEvent::ToolCallStart { id, name } 
            if id == "call_abc" && name == "bash"));
    }

    #[test]
    fn llm_error_from_provider_error() {
        let err = ProviderError::MissingApiKey("openai".into());
        let llm_err: LLMError = err.into();
        assert!(matches!(llm_err, LLMError::Other(s) if s.contains("openai")));
    }

    #[test]
    fn stop_reason_serialization() {
        let reason = StopReason::ToolCalls;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, "\"tool_calls\"");
    }
}
