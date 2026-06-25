//! Provider trait and types

use crate::message::ChatMessage;
use crate::provider_event::ProviderEvent;
use anyhow::Result;
use futures::Stream;
use std::pin::Pin;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// The provider key was not found in the registry.
    UnknownProvider(String),
    /// The API key is missing or invalid for this provider.
    MissingApiKey(String),
    /// Configuration has not been loaded yet.
    ConfigNotLoaded,
    /// Some other error during construction or API call.
    Other(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::UnknownProvider(k) => write!(f, "Unknown provider: {}", k),
            ProviderError::MissingApiKey(k) => {
                let provider = k
                    .strip_suffix("_API_KEY")
                    .map(|p| p.to_lowercase())
                    .unwrap_or_else(|| k.to_lowercase());
                write!(
                    f,
                    "Missing API key for {}. Set {} or add [model_providers.{}] api_key to ~/.runie/config.toml",
                    provider, k, provider
                )
            }
            ProviderError::ConfigNotLoaded => write!(f, "Configuration not loaded"),
            ProviderError::Other(s) => write!(f, "Provider error: {}", s),
        }
    }
}

impl std::error::Error for ProviderError {}

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
}
