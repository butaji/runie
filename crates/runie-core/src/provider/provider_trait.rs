//! Provider trait and types

use crate::message::ChatMessage;
use crate::provider_event::ProviderEvent;
use anyhow::Result;
use futures::Stream;
use std::pin::Pin;
use thiserror::Error;

/// A chunk of streaming response (legacy type, prefer ProviderEvent).
#[derive(Debug, Clone)]
pub struct ResponseChunk {
    pub content: String,
}

impl From<ResponseChunk> for ProviderEvent {
    fn from(chunk: ResponseChunk) -> Self {
        ProviderEvent::TextDelta(chunk.content)
    }
}

/// Error constructing or operating a provider.
///
/// Variants carry typed data so callers can match on specific error kinds
/// (rate limit, network, auth, etc.) rather than string-matching error messages.
/// The `Source` variant is the catch-all for unexpected errors.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// The provider key was not found in the registry.
    #[error("Unknown provider: {0}")]
    UnknownProvider(String),
    /// The API key is missing or invalid for this provider.
    #[error(transparent)]
    MissingApiKey(MissingApiKeyError),
    /// Configuration has not been loaded yet.
    #[error("Configuration not loaded")]
    ConfigNotLoaded,
    /// Rate limit hit — retry info available via struct fields.
    #[error("Rate limited (retry after {retry_after_secs:?}s)")]
    RateLimit { retry_after_secs: Option<u32> },
    /// Network connectivity error (connection refused, DNS failure, etc.).
    #[error("Network error: {0}")]
    Network(String),
    /// Request timed out.
    #[error("Request timed out")]
    Timeout,
    /// Server error (HTTP 5xx) with status code and optional message.
    #[error("Server error {0}{1}")]
    Server(u16, String),
    /// Authentication / authorisation failed (HTTP 401 / 403).
    #[error("Authentication failed ({0})")]
    Auth(u16),
    /// Context length limit exceeded.
    #[error("Context length exceeded: {0} tokens")]
    ContextLength(usize),
    /// An underlying error that does not fit a typed variant.
    /// `#[transparent]` forwards `Display` and `Error::source()` to the inner `anyhow::Error`.
    #[error(transparent)]
    Source(anyhow::Error),
}

impl Clone for ProviderError {
    fn clone(&self) -> Self {
        use ProviderError::*;
        match self {
            UnknownProvider(s) => UnknownProvider(s.clone()),
            MissingApiKey(e) => MissingApiKey((*e).clone()),
            ConfigNotLoaded => ConfigNotLoaded,
            RateLimit { retry_after_secs } => RateLimit { retry_after_secs: *retry_after_secs },
            Network(s) => Network(s.clone()),
            Timeout => Timeout,
            Server(code, msg) => Server(*code, msg.clone()),
            Auth(code) => Auth(*code),
            ContextLength(n) => ContextLength(*n),
            // anyhow::Error is not Clone — store formatted message as a new error
            Source(e) => Source(anyhow::anyhow!("{e}")),
        }
    }
}

impl ProviderError {
    /// Returns true if this error is retryable (transient, not fatal).
    pub fn is_retryable(&self) -> bool {
        match self {
            // Fatal — credentials are wrong
            ProviderError::Auth(_) => false,
            // Fatal — provider is unknown or config is missing
            ProviderError::UnknownProvider(_) | ProviderError::MissingApiKey(_) => false,
            ProviderError::ConfigNotLoaded => false,
            // Fatal — prompt is too long
            ProviderError::ContextLength(_) => false,
            // Transient — retry
            ProviderError::RateLimit { .. } => true,
            ProviderError::Network(_) => true,
            ProviderError::Timeout => true,
            ProviderError::Server(_, _) => true,
            // Unknown — conservative: retry
            ProviderError::Source(_) => true,
        }
    }

    /// Returns true if this error is fatal and should not be retried.
    pub fn is_fatal(&self) -> bool {
        !self.is_retryable()
    }

    /// Classify a reqwest error into a typed `ProviderError` variant.
    pub fn from_reqwest(err: &reqwest::Error) -> Self {
        if let Some(status) = err.status() {
            let code = status.as_u16();
            if code == 401 || code == 403 {
                return ProviderError::Auth(code);
            }
            if code == 429 {
                // Retry-After can be extracted via the response body or headers when available
                return ProviderError::RateLimit { retry_after_secs: None };
            }
            if code >= 500 {
                return ProviderError::Server(code, Default::default());
            }
            // 4xx other than 401/403/429 — wrap as source error
            return ProviderError::Source(anyhow::anyhow!("{}", err));
        }
        if err.is_timeout() {
            return ProviderError::Timeout;
        }
        if err.is_connect() {
            return ProviderError::Network(err.to_string());
        }
        // Fallback: wrap the error
        ProviderError::Source(anyhow::anyhow!("{}", err))
    }

}

/// Helper error for displaying missing API key errors.
#[derive(Debug, Error, Clone)]
#[error("Missing API key for {provider}. Set {env_var} or add [model_providers.{provider}] api_key to ~/.runie/config.toml")]
pub struct MissingApiKeyError {
    pub env_var: String,
    pub provider: String,
}

impl From<String> for MissingApiKeyError {
    fn from(k: String) -> Self {
        let env_var = k.clone();
        let provider = k
            .strip_suffix("_API_KEY")
            .map(|p| p.to_lowercase())
            .unwrap_or_else(|| k.to_lowercase());
        MissingApiKeyError { env_var, provider }
    }
}

impl From<&str> for MissingApiKeyError {
    fn from(k: &str) -> Self {
        k.to_string().into()
    }
}

impl From<String> for ProviderError {
    fn from(k: String) -> Self {
        ProviderError::MissingApiKey(k.into())
    }
}

impl From<anyhow::Error> for ProviderError {
    fn from(e: anyhow::Error) -> Self {
        // Attempt to extract a typed variant from anyhow errors that wrap typed errors
        if let Some(typed) = e.downcast_ref::<ProviderError>() {
            // Already typed — return it directly
            return typed.clone();
        }
        // Fall back to wrapping in Source
        ProviderError::Source(e)
    }
}

/// Provider trait — implemented by LLM backends.
/// Returns a `Stream` of `ProviderEvent`s.
///
/// This trait is dyn-compatible (no `async fn`, no generic parameters).
pub trait Provider: Send + Sync {
    /// Generate a streaming response, returning a stream of LLM events.
    fn generate(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = Result<ProviderEvent>> + Send + '_>>;

    /// Generate with a set of tool definitions that the model may invoke.
    ///
    /// Providers that do not support native tool calling can ignore `tools`
    /// and fall back to `generate`. The default implementation does exactly
    /// that, preserving backward compatibility for existing providers.
    fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        _tools: Vec<serde_json::Value>,
    ) -> Pin<Box<dyn Stream<Item = Result<ProviderEvent>> + Send + '_>> {
        self.generate(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Layer 1: typed error variant displays ─────────────────────────────────

    #[test]
    fn missing_api_key_display_names_provider_and_env_var() {
        let err = ProviderError::MissingApiKey("MINIMAX_API_KEY".into());
        let msg = err.to_string();
        assert!(msg.contains("minimax"), "{msg}");
        assert!(msg.contains("MINIMAX_API_KEY"), "{msg}");
        assert!(msg.contains("[model_providers.minimax]"), "{msg}");
    }

    #[test]
    fn typed_error_display_rate_limit() {
        let err = ProviderError::RateLimit { retry_after_secs: Some(60) };
        let msg = err.to_string();
        assert!(msg.contains("Rate limited"), "{msg}");
        assert!(err.is_retryable());
        assert!(!err.is_fatal());
    }

    #[test]
    fn typed_error_display_network() {
        let err = ProviderError::Network("connection refused".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Network error"), "{msg}");
        assert!(err.is_retryable());
        assert!(!err.is_fatal());
    }

    #[test]
    fn typed_error_display_timeout() {
        let err = ProviderError::Timeout;
        let msg = err.to_string();
        assert!(msg.contains("timed out"), "{msg}");
        assert!(err.is_retryable());
        assert!(!err.is_fatal());
    }

    #[test]
    fn typed_error_display_server() {
        let err = ProviderError::Server(502, "Bad Gateway".to_string());
        let msg = err.to_string();
        assert!(msg.contains("502"), "{msg}");
        assert!(err.is_retryable());
        assert!(!err.is_fatal());
    }

    #[test]
    fn typed_error_display_auth() {
        let err = ProviderError::Auth(401);
        let msg = err.to_string();
        assert!(msg.contains("Authentication failed"), "{msg}");
        assert!(msg.contains("401"), "{msg}");
        assert!(!err.is_retryable());
        assert!(err.is_fatal());
    }

    #[test]
    fn typed_error_display_context_length() {
        let err = ProviderError::ContextLength(128_000);
        let msg = err.to_string();
        assert!(msg.contains("Context length exceeded"), "{msg}");
        assert!(msg.contains("128000"), "{msg}");
        assert!(!err.is_retryable());
        assert!(err.is_fatal());
    }

    // ─── Layer 1: is_retryable determinism ─────────────────────────────────────

    #[test]
    fn retryable_is_true_for_transient_errors() {
        let transient = [
            ProviderError::RateLimit { retry_after_secs: None },
            ProviderError::Network("connection refused".into()),
            ProviderError::Timeout,
            ProviderError::Server(500, Default::default()),
            ProviderError::Server(503, "Service Unavailable".into()),
        ];
        for err in transient {
            assert!(
                err.is_retryable(),
                "expected {err:?} to be retryable"
            );
        }
    }

    #[test]
    fn retryable_is_false_for_fatal_errors() {
        let fatal = [
            ProviderError::Auth(401),
            ProviderError::Auth(403),
            ProviderError::ContextLength(100_000),
            ProviderError::UnknownProvider("foo".into()),
            ProviderError::MissingApiKey("OPENAI_API_KEY".into()),
            ProviderError::ConfigNotLoaded,
        ];
        for err in fatal {
            assert!(
                err.is_retryable() == false,
                "expected {err:?} to NOT be retryable"
            );
            assert!(
                err.is_fatal(),
                "expected {err:?} to be fatal"
            );
        }
    }

    #[test]
    fn clone_preserves_variant_and_data() {
        let err = ProviderError::Server(503, "Service Unavailable".into());
        let cloned = err.clone();
        assert!(matches!(cloned, ProviderError::Server(503, msg) if msg == "Service Unavailable"));

        let auth_err = ProviderError::Auth(401);
        assert!(matches!(auth_err.clone(), ProviderError::Auth(401)));

        let rate_err = ProviderError::RateLimit { retry_after_secs: Some(30) };
        assert!(matches!(rate_err.clone(), ProviderError::RateLimit { retry_after_secs: Some(30) }));
    }

    // ─── Layer 1: existing error display messages are preserved ────────────────

    #[test]
    fn central_error_displays_preserve_messages() {
        let cases = [
            (
                ProviderError::UnknownProvider("my-model".into()),
                "Unknown provider: my-model",
            ),
            (
                ProviderError::MissingApiKey("OPENAI_API_KEY".into()),
                "Missing API key",
            ),
            (ProviderError::ConfigNotLoaded, "Configuration not loaded"),
        ];
        for (err, prefix) in cases {
            let msg = err.to_string();
            assert!(
                msg.starts_with(prefix),
                "expected message to start with '{prefix}', got: {msg}"
            );
        }
    }

    // Layer 1: provider errors are still identifiable by variant
    #[test]
    fn provider_error_source_round_trips() {
        let anyhow_err = anyhow::anyhow!("network error: connection refused");
        let err: ProviderError = anyhow_err.into();
        let msg = err.to_string();
        // The underlying error message is preserved in the display
        assert!(
            msg.contains("network error"),
            "expected 'network error' in: {msg}"
        );
        assert!(
            msg.contains("connection refused"),
            "expected 'connection refused' in: {msg}"
        );
        // The variant is still Source
        assert!(
            matches!(err, ProviderError::Source(_)),
            "expected Source variant, got: {err:?}"
        );
    }
}
