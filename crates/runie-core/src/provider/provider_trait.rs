//! Provider trait and types

use crate::message::ChatMessage;
use crate::model_catalog::{ModelCapabilities, ModelInfo};
use crate::provider_event::ProviderEvent;
use anyhow::Result;
use derive_builder::Builder;
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
    #[error(
        "Rate limited{}",
        .retry_after_secs.map(|s| format!(" (retry after {s}s)")).unwrap_or_default()
    )]
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
    /// Client error (HTTP 4xx other than auth/rate-limit) with status and message.
    #[error("Bad request {0}{1}")]
    BadRequest(u16, String),
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
            BadRequest(code, msg) => BadRequest(*code, msg.clone()),
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
        } else if code == 408 {
            // HTTP 408 Request Timeout is transient — retry.
            Some(ProviderError::Server(code, Default::default()))
        } else if code == 429 {
            Some(ProviderError::RateLimit { retry_after_secs: None })
        } else if code >= 500 {
            Some(ProviderError::Server(code, Default::default()))
        } else if code >= 400 {
            // All other 4xx client errors (including 400 Bad Request) are fatal.
            Some(ProviderError::BadRequest(code, Default::default()))
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
            // Fatal — client error (e.g. 400 Bad Request)
            ProviderError::BadRequest(_, _) => false,
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
            if let Some(mut typed) = ProviderError::classify_http_status(code) {
                // Preserve the original error message on variants that carry one.
                if let ProviderError::Server(_, ref mut msg) = typed {
                    *msg = err.to_string();
                } else if let ProviderError::BadRequest(_, ref mut msg) = typed {
                    *msg = err.to_string();
                }
                return typed;
            }
            // Non-error status codes should not reach here, but keep a fallback.
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
#[error(
    "Missing API key for {provider}. Set {env_var} or add [model_providers.{provider}] api_key to ~/.runie/config.toml"
)]
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
        Self { max_attempts, initial_delay, max_delay, multiplier }
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

    /// Create a RetryPolicy from this RetryConfig with no per-error-type overrides.
    pub fn into_policy(self) -> RetryPolicy {
        RetryPolicy {
            base: self,
            rate_limit_retries: None,
            timeout_retries: None,
            context_window_retries: None,
            bad_request_retries: None,
        }
    }
}

/// Per-error-type retry policy overrides.
///
/// When a specific retry count is set for an error type, it overrides
/// the base `RetryConfig.max_attempts` for that error type.
#[derive(Clone, Debug, PartialEq)]
pub struct RetryPolicy {
    /// Base retry configuration.
    pub base: RetryConfig,
    /// Override retry count for rate limit errors (429).
    pub rate_limit_retries: Option<u32>,
    /// Override retry count for timeout errors.
    pub timeout_retries: Option<u32>,
    /// Override retry count for context window exceeded errors.
    /// Note: ContextLength errors are fatal (non-retryable) by default.
    pub context_window_retries: Option<u32>,
    /// Override retry count for bad request errors.
    /// Note: BadRequest errors are fatal (non-retryable) by default.
    pub bad_request_retries: Option<u32>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryConfig::default().into_policy()
    }
}

impl RetryPolicy {
    /// Create a new retry policy with default settings and per-error-type overrides.
    pub fn new(
        base: RetryConfig,
        rate_limit_retries: Option<u32>,
        timeout_retries: Option<u32>,
        context_window_retries: Option<u32>,
        bad_request_retries: Option<u32>,
    ) -> Self {
        Self { base, rate_limit_retries, timeout_retries, context_window_retries, bad_request_retries }
    }

    /// Get the retry count for a typed ProviderError.
    /// Returns the base config's max_attempts if no override is set.
    pub fn max_attempts_for_error(&self, error: &ProviderError) -> u32 {
        match error {
            ProviderError::RateLimit { .. } => self.rate_limit_retries.unwrap_or(self.base.max_attempts),
            ProviderError::Timeout => self.timeout_retries.unwrap_or(self.base.max_attempts),
            ProviderError::ContextLength(_) => self
                .context_window_retries
                .unwrap_or(self.base.max_attempts),
            ProviderError::BadRequest(_, _) => self.bad_request_retries.unwrap_or(self.base.max_attempts),
            // For other retryable errors (Server, Network, Source), use base config
            _ if error.is_retryable() => self.base.max_attempts,
            // Fatal errors use base config (though they won't be retried anyway)
            _ => self.base.max_attempts,
        }
    }
}

/// Provider trait — implemented by LLM backends.
/// Returns a `Stream` of `ProviderEvent`s.
///
/// This trait is dyn-compatible (no `async fn`, no generic parameters).
pub trait Provider: Send + Sync {
    /// Generate a streaming response, returning a stream of LLM events.
    fn generate(&self, messages: Vec<ChatMessage>) -> Pin<Box<dyn Stream<Item = Result<ProviderEvent>> + Send + '_>>;

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
mod tests;
