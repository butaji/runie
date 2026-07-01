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
///
/// Uses `thiserror` to derive `Display` and `Error` with proper `#[source]` chains.
/// The `Other` variant stores the underlying `anyhow::Error` directly so that the
/// full source chain (`Error::source()`) is preserved for error introspection.
/// Serialization formats the error message into the JSON `message` field.
#[derive(Debug, Error)]
pub enum ModelError {
    /// LLM returned invalid JSON.
    #[error("JSON decode error: {0}")]
    JsonDecode(String),
    /// Token limit exceeded.
    #[error("Context length exceeded: used {used}, limit {limit}")]
    ContextLength { limit: usize, used: usize },
    /// Model refused the request.
    #[error("Model refused: {0}")]
    Refusal(String),
    /// Rate limit hit (retry info available via struct fields).
    #[error("Rate limited")]
    RateLimit { retry_after_secs: Option<u32> },
    /// An underlying error (network, parse, etc.) with preserved source chain.
    /// `#[transparent]` forwards Display and Error::source to the inner anyhow::Error.
    #[error(transparent)]
    Other(anyhow::Error),
}

// ─── Manual trait impls that thiserror can't derive ──────────────────────────

impl Clone for ModelError {
    fn clone(&self) -> Self {
        use ModelError::*;
        match self {
            JsonDecode(s) => JsonDecode(s.clone()),
            ContextLength { limit, used } => ContextLength { limit: *limit, used: *used },
            Refusal(s) => Refusal(s.clone()),
            RateLimit { retry_after_secs } => RateLimit { retry_after_secs: *retry_after_secs },
            // anyhow::Error is not Clone, so we clone the message string instead
            Other(e) => Other(anyhow::anyhow!("{e}")),
        }
    }
}

impl PartialEq for ModelError {
    fn eq(&self, other: &Self) -> bool {
        use ModelError::*;
        match (self, other) {
            (JsonDecode(a), JsonDecode(b)) => a == b,
            (ContextLength { limit: al, used: au }, ContextLength { limit: bl, used: bu }) => al == bl && au == bu,
            (Refusal(a), Refusal(b)) => a == b,
            (RateLimit { retry_after_secs: a }, RateLimit { retry_after_secs: b }) => a == b,
            // anyhow::Error does not impl PartialEq; compare formatted messages
            (Other(a), Other(b)) => a.to_string() == b.to_string(),
            _ => false,
        }
    }
}

impl serde::Serialize for ModelError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ModelError", 3)?;
        match self {
            ModelError::JsonDecode(msg) => {
                s.serialize_field("kind", "jsonDecode")?;
                s.serialize_field("message", &format!("JSON decode error: {msg}"))?;
            }
            ModelError::ContextLength { limit, used } => {
                s.serialize_field("kind", "contextLength")?;
                s.serialize_field("message", &format!("Context length exceeded: used {used}, limit {limit}"))?;
            }
            ModelError::Refusal(msg) => {
                s.serialize_field("kind", "refusal")?;
                s.serialize_field("message", &format!("Model refused: {msg}"))?;
            }
            ModelError::RateLimit { retry_after_secs } => {
                s.serialize_field("kind", "rateLimit")?;
                s.serialize_field("message", "Rate limited")?;
                s.serialize_field("retryAfterSecs", retry_after_secs)?;
            }
            ModelError::Other(e) => {
                s.serialize_field("kind", "other")?;
                s.serialize_field("message", &e.to_string())?;
            }
        }
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for ModelError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ModelErrorJson {
            kind: String,
            message: String,
            #[serde(default)]
            retry_after_secs: Option<u32>,
        }
        let json = ModelErrorJson::deserialize(deserializer)?;
        match json.kind.as_str() {
            "jsonDecode" => Ok(ModelError::JsonDecode(json.message.trim_start_matches("JSON decode error: ").to_owned())),
            "contextLength" => {
                // Parse "used X, limit Y" format
                let used = json.message.split(", limit ").next()
                    .and_then(|s| s.split("used ").nth(1))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_default();
                let limit = json.message.split(", limit ").nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_default();
                Ok(ModelError::ContextLength { limit, used })
            }
            "refusal" => Ok(ModelError::Refusal(json.message.trim_start_matches("Model refused: ").to_owned())),
            "rateLimit" => Ok(ModelError::RateLimit { retry_after_secs: json.retry_after_secs }),
            "other" => Ok(ModelError::Other(anyhow::anyhow!("{}", json.message))),
            _ => Ok(ModelError::Other(anyhow::anyhow!("{}", json.message))),
        }
    }
}

impl From<ProviderError> for ModelError {
    fn from(e: ProviderError) -> Self {
        // Wrap in anyhow::Error to preserve the full source chain
        ModelError::Other(anyhow::Error::from(e))
    }
}

impl From<anyhow::Error> for ModelError {
    fn from(e: anyhow::Error) -> Self {
        ModelError::Other(e)
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
            let inner = std::io::Error::new(std::io::ErrorKind::Other, "propagated");
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
}
