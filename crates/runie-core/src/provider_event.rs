//! Canonical provider event vocabulary.
//!
//! Normalizes events from all providers (OpenAI, Anthropic, Ollama, etc.)
//! into a unified vocabulary distinct from the internal `Event` type.

use crate::provider::ProviderError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
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

/// Model-specific errors (distinct from `ProviderError`).
///
/// Uses `thiserror` to derive `Display` and `Error` with proper `#[source]` chains.
/// The `Other` variant stores the error message as a `String` for simple serialization;
/// the original source chain is lost in this representation but the user-visible
/// message is preserved.
#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum ModelError {
    /// LLM returned invalid JSON.
    #[error("JSON decode error: {0}")]
    JsonDecode(String),
    /// Token limit exceeded.
    #[error("Context length exceeded: used {used}, limit {limit}")]
    ContextLength {
        limit: usize,
        used: usize,
    },
    /// Model refused the request.
    #[error("Model refused: {0}")]
    Refusal(String),
    /// Rate limit hit (retry info available via struct fields).
    #[error("Rate limited")]
    RateLimit {
        #[serde(
            rename = "retryAfterSecs",
            skip_serializing_if = "Option::is_none"
        )]
        retry_after_secs: Option<u32>,
    },
    /// An underlying error (network, parse, etc.).
    /// Stores only the message string for serialization simplicity.
    #[error("{0}")]
    Other(String),
}

// ─── From implementations ───────────────────────────────────────────────────────

impl From<ProviderError> for ModelError {
    fn from(e: ProviderError) -> Self {
        use ProviderError::*;
        match e {
            // Typed: RateLimit preserves retry info (e.g., Retry-After header).
            RateLimit { retry_after_secs } => ModelError::RateLimit { retry_after_secs },
            // Typed: ContextLength — provider reports the limit; use it as both limit and used.
            ContextLength(n) => ModelError::ContextLength { limit: n, used: n },
            // All other provider errors fall through to Other, preserving the full message.
            _ => ModelError::Other(e.to_string()),
        }
    }
}

impl From<anyhow::Error> for ModelError {
    fn from(e: anyhow::Error) -> Self {
        // Store the error message as a string for serialization
        ModelError::Other(e.to_string())
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
        let msg = model_err.to_string();
        assert!(msg.contains("openai"), "expected 'openai' in error message: {msg}");
    }

    #[test]
    fn model_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("network timeout");
        let model_err: ModelError = anyhow_err.into();
        let msg = model_err.to_string();
        assert!(msg.contains("network timeout"), "expected 'network timeout' in error message: {msg}");
    }

    #[test]
    fn model_error_other_preserves_error_chain_via_propagation() {
        // Verify that errors can be converted via `?` propagation through ModelError::Other
        let inner = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        // anyhow::Error::from() wraps the inner error so it can be propagated via `?`
        let model_err: ModelError = anyhow::Error::from(inner).into();
        // The error message is preserved (what users see)
        let msg = model_err.to_string();
        assert!(msg.contains("file not found") || msg.contains("NotFound"),
            "expected file-not-found error message, got: {msg}");
        // Verify that ? propagation works (errors can be converted through ModelError)
        fn propagate_error() -> Result<(), ModelError> {
            let inner = std::io::Error::other("propagated");
            Err(anyhow::Error::from(inner))?;
            Ok(())
        }
        let result = propagate_error();
        assert!(result.is_err(), "expected error propagation to work");
    }

    #[test]
    fn stop_reason_serialization() {
        let reason = StopReason::ToolCalls;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, "\"tool_calls\"");
    }

    // ─── ModelError JSON round-trip tests (Layer 1) ───────────────────────────

    #[test]
    fn model_error_json_decode_roundtrip() {
        let err = ModelError::JsonDecode("unexpected token".into());
        let json = serde_json::to_string(&err).unwrap();
        // With #[serde(tag = "kind", content = "message")], tuple variants put content directly in message
        assert_eq!(json, r#"{"kind":"jsonDecode","message":"unexpected token"}"#);
        let roundtrip: ModelError = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, err);
    }

    #[test]
    fn model_error_context_length_roundtrip() {
        let err = ModelError::ContextLength { limit: 8192, used: 9000 };
        let json = serde_json::to_string(&err).unwrap();
        // Struct variants put the struct fields in message
        assert_eq!(
            json,
            r#"{"kind":"contextLength","message":{"limit":8192,"used":9000}}"#
        );
        let roundtrip: ModelError = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, err);
    }

    #[test]
    fn model_error_refusal_roundtrip() {
        let err = ModelError::Refusal("content blocked".into());
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, r#"{"kind":"refusal","message":"content blocked"}"#);
        let roundtrip: ModelError = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, err);
    }

    #[test]
    fn model_error_rate_limit_roundtrip() {
        let err = ModelError::RateLimit { retry_after_secs: Some(30) };
        let json = serde_json::to_string(&err).unwrap();
        // The retry_after_secs field is renamed to retryAfterSecs in JSON
        assert_eq!(json, r#"{"kind":"rateLimit","message":{"retryAfterSecs":30}}"#);
        let roundtrip: ModelError = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, err);

        // Without retry info, the field is skipped entirely
        let err2 = ModelError::RateLimit { retry_after_secs: None };
        let json2 = serde_json::to_string(&err2).unwrap();
        assert_eq!(json2, r#"{"kind":"rateLimit","message":{}}"#);
        let roundtrip2: ModelError = serde_json::from_str(&json2).unwrap();
        assert_eq!(roundtrip2, err2);
    }

    #[test]
    fn model_error_other_roundtrip() {
        let err = ModelError::Other("connection refused".into());
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, r#"{"kind":"other","message":"connection refused"}"#);
        let roundtrip: ModelError = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, err);
    }

    #[test]
    fn provider_event_error_roundtrip() {
        let event = ProviderEvent::Error(ModelError::Refusal("policy violation".into()));
        let json = serde_json::to_string(&event).unwrap();
        // New format: message contains just the refusal text
        assert_eq!(
            json,
            r#"{"type":"error","data":{"kind":"refusal","message":"policy violation"}}"#
        );
        let roundtrip: ProviderEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, event);
    }
}
