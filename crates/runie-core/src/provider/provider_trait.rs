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
    /// An underlying error from the provider or network layer.
    #[error("Provider error: {0}")]
    Source(#[source] anyhow::Error),
}

/// Helper error for displaying missing API key errors.
#[derive(Debug, Error)]
pub struct MissingApiKeyError {
    pub env_var: String,
    pub provider: String,
}

impl std::fmt::Display for MissingApiKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Missing API key for {}. Set {} or add [model_providers.{}] api_key to ~/.runie/config.toml",
            self.provider, self.env_var, self.provider
        )
    }
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

    #[test]
    fn missing_api_key_display_names_provider_and_env_var() {
        let err = ProviderError::MissingApiKey("MINIMAX_API_KEY".into());
        let msg = err.to_string();
        assert!(msg.contains("minimax"), "{msg}");
        assert!(msg.contains("MINIMAX_API_KEY"), "{msg}");
        assert!(msg.contains("[model_providers.minimax]"), "{msg}");
    }

    // Layer 1: existing error display messages are preserved
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
            (
                ProviderError::ConfigNotLoaded,
                "Configuration not loaded",
            ),
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
        assert!(msg.contains("network error"), "expected 'network error' in: {msg}");
        assert!(msg.contains("connection refused"), "expected 'connection refused' in: {msg}");
        // The variant is still Source
        assert!(
            matches!(err, ProviderError::Source(_)),
            "expected Source variant, got: {err:?}"
        );
    }
}
