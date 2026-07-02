//! Provider trait and types

use crate::message::ChatMessage;
use derive_builder::Builder;
use crate::model_catalog::{ModelCapabilities, ModelInfo};
use crate::provider_event::ProviderEvent;
use anyhow::Result;
use futures::Stream;
use std::pin::Pin;
use std::time::Duration;
use thiserror::Error;

/// Default retry configuration for provider streams.
/// Used when no explicit retry config is provided.
pub const DEFAULT_RETRY_CONFIG: RetryConfig = RetryConfig {
    max_attempts: 5,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(30),
    multiplier: 2.0,
};

/// Default HTTP request timeout (120 s).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Default HTTP connect timeout (10 s).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

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
    /// Classify an HTTP status code into a typed `ProviderError` variant.
    /// Returns `None` if the status code doesn't warrant a typed error.
    pub fn classify_http_status(code: u16) -> Option<Self> {
        if code == 401 || code == 403 {
            Some(ProviderError::Auth(code))
        } else if code == 429 {
            Some(ProviderError::RateLimit { retry_after_secs: None })
        } else if code >= 500 {
            Some(ProviderError::Server(code, Default::default()))
        } else {
            None
        }
    }

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
            if let Some(typed) = ProviderError::classify_http_status(code) {
                return typed;
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

/// Metadata about a provider's capabilities and configuration.
/// Used by consumers to discover model info, streaming support, and retry settings.
#[derive(Clone, Debug, Default, Builder)]
#[builder(setter(strip_option))]
pub struct ProviderMetadata {
    /// Model-specific information (name, provider, costs, context window).
    #[builder(default, setter(strip_option))]
    pub model_info: Option<ModelInfo>,
    /// Computed capabilities derived from model_info.
    pub capabilities: ModelCapabilities,
    /// Retry configuration for transient failures.
    pub retry_config: RetryConfig,
    /// Whether this provider supports streaming responses.
    /// Defaults to `true` if not specified in model_info.
    pub streaming: bool,
    /// Whether this provider supports native tool calling.
    /// Defaults to `true` if not specified in model_info.
    pub supports_tools: bool,
}

impl ProviderMetadata {
    /// Create new metadata with default retry config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model info and derive capabilities from it.
    pub fn with_model_info(mut self, info: ModelInfo) -> Self {
        self.model_info = Some(info.clone());
        self.capabilities = info.capabilities.clone();
        self.streaming = self.capabilities.streaming;
        self.supports_tools = self.capabilities.supports_tools;
        self
    }

    /// Set the retry configuration.
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Set whether streaming is supported.
    pub fn with_streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    /// Set whether tools are supported.
    pub fn with_supports_tools(mut self, supports_tools: bool) -> Self {
        self.supports_tools = supports_tools;
        self
    }
}

/// Configuration for retry behavior on transient provider errors.
#[derive(Clone, Debug, PartialEq, Builder)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Initial delay before first retry.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Multiplier for exponential backoff (e.g., 2.0 doubles delay each attempt).
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        DEFAULT_RETRY_CONFIG
    }
}

impl RetryConfig {
    /// Create a new retry config with custom settings.
    pub fn new(max_attempts: u32, initial_delay: Duration, max_delay: Duration, multiplier: f64) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay,
            multiplier,
        }
    }

    /// Create a config that disables retries (max_attempts = 1).
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            initial_delay: Duration::from_secs(0),
            max_delay: Duration::from_secs(0),
            multiplier: 1.0,
        }
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

    /// Return metadata about this provider's capabilities.
    ///
    /// The default implementation returns metadata derived from the model info
    /// if available, with streaming and tool support inferred from capabilities.
    /// Override this method to provide custom metadata or disable features.
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata::default()
    }

    /// Generate a non-streaming (fast) response for models that don't support streaming.
    ///
    /// This is useful for models like o1 that don't support streaming responses.
    /// The default implementation wraps `generate` and collects all events into a single response.
    /// Providers that natively support non-streaming should override this method.
    fn complete_fast(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = Result<ProviderEvent>> + Send + '_>> {
        self.generate(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_catalog::ModelCapabilitiesBuilder;

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

    // ─── Layer 1: ProviderMetadata tests ────────────────────────────────────────

    #[test]
    fn provider_metadata_default_values() {
        let meta = ProviderMetadata::default();
        assert!(meta.model_info.is_none());
        assert!(!meta.streaming);
        assert!(!meta.supports_tools);
        assert_eq!(meta.retry_config.max_attempts, DEFAULT_RETRY_CONFIG.max_attempts);
    }

    #[test]
    fn provider_metadata_with_model_info() {
        let info = ModelInfo::new("openai", "gpt-4o");
        let meta = ProviderMetadata::new().with_model_info(info);
        assert!(meta.model_info.is_some());
        assert_eq!(meta.model_info.as_ref().unwrap().name, "gpt-4o");
        assert_eq!(meta.model_info.as_ref().unwrap().provider, "openai");
    }

    #[test]
    fn provider_metadata_with_custom_retry_config() {
        let custom_config = RetryConfig::new(10, Duration::from_secs(1), Duration::from_secs(60), 3.0);
        let meta = ProviderMetadata::new().with_retry_config(custom_config.clone());
        assert_eq!(meta.retry_config.max_attempts, 10);
        assert_eq!(meta.retry_config.multiplier, 3.0);
    }

    #[test]
    fn provider_metadata_streaming_flag() {
        let meta = ProviderMetadata::new().with_streaming(true);
        assert!(meta.streaming);

        let meta = ProviderMetadata::new().with_streaming(false);
        assert!(!meta.streaming);
    }

    #[test]
    fn provider_metadata_supports_tools_flag() {
        let meta = ProviderMetadata::new().with_supports_tools(true);
        assert!(meta.supports_tools);

        let meta = ProviderMetadata::new().with_supports_tools(false);
        assert!(!meta.supports_tools);
    }

    // ─── Layer 1: RetryConfig tests ────────────────────────────────────────────

    #[test]
    fn retry_config_default_values() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.multiplier, 2.0);
    }

    #[test]
    fn retry_config_no_retry() {
        let config = RetryConfig::no_retry();
        assert_eq!(config.max_attempts, 1);
        assert_eq!(config.initial_delay, Duration::from_secs(0));
        assert_eq!(config.max_delay, Duration::from_secs(0));
        assert_eq!(config.multiplier, 1.0);
    }

    #[test]
    fn retry_config_custom_values() {
        let config = RetryConfig::new(3, Duration::from_secs(1), Duration::from_secs(10), 1.5);
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(10));
        assert_eq!(config.multiplier, 1.5);
    }

    #[test]
    fn retry_config_clone_preserves_values() {
        let config = RetryConfig::new(7, Duration::from_secs(2), Duration::from_secs(120), 4.0);
        let cloned = config.clone();
        assert_eq!(cloned.max_attempts, config.max_attempts);
        assert_eq!(cloned.initial_delay, config.initial_delay);
        assert_eq!(cloned.max_delay, config.max_delay);
        assert_eq!(cloned.multiplier, config.multiplier);
    }

    #[test]
    fn retry_config_partial_eq() {
        let config1 = RetryConfig::new(5, Duration::from_secs(1), Duration::from_secs(30), 2.0);
        let config2 = RetryConfig::new(5, Duration::from_secs(1), Duration::from_secs(30), 2.0);
        let config3 = RetryConfig::new(6, Duration::from_secs(1), Duration::from_secs(30), 2.0);
        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    #[test]
    fn retry_config_derive_builder() {
        // Exercise the derive_builder generated API for RetryConfig.
        // derive_builder generates StructNameBuilder (not StructName::builder()).
        let config = RetryConfigBuilder::default()
            .max_attempts(10)
            .initial_delay(Duration::from_millis(500))
            .max_delay(Duration::from_secs(60))
            .multiplier(2.0)
            .build()
            .unwrap();
        assert_eq!(config.max_attempts, 10);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(60));
        assert_eq!(config.multiplier, 2.0);
    }

    #[test]
    fn provider_metadata_derive_builder() {
        // Exercise the derive_builder generated API for ProviderMetadata.
        // ModelCapabilities requires all its fields when built via derive_builder.
        // ProviderMetadata fields model_info and retry_config are provided explicitly.
        let caps = ModelCapabilitiesBuilder::default()
            .streaming(true)
            .supports_vision(true)
            .supports_tools(true)
            .supports_reasoning(false)
            .max_context_tokens(128_000)
            .max_output_tokens(8_192)
            .cache_control(false)
            .build()
            .unwrap();
        let retry = RetryConfigBuilder::default()
            .max_attempts(5)
            .initial_delay(Duration::from_millis(100))
            .max_delay(Duration::from_secs(30))
            .multiplier(2.0)
            .build()
            .unwrap();
        let meta = ProviderMetadataBuilder::default()
            .capabilities(caps)
            .retry_config(retry)
            .streaming(true)
            .supports_tools(true)
            .build()
            .unwrap();
        assert!(meta.streaming);
        assert!(meta.supports_tools);
        assert!(meta.capabilities.streaming);
        assert!(meta.capabilities.supports_vision);
        assert!(meta.capabilities.supports_tools);
    }
}
