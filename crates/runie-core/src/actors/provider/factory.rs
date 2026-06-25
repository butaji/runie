//! Abstract factory used by `ProviderActor` to build and validate providers.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::config::Config;
use crate::provider_event::ProviderEvent;
use crate::message::ChatMessage;
use crate::provider::{Provider, ProviderError};

/// Built provider that wraps a concrete provider implementation.
///
/// This type implements `Provider` directly, so it can be used anywhere a
/// `Box<dyn Provider>` is expected without an extra layer of indirection.
/// The `DynProvider` type in `runie-provider` is a type alias for this struct.
#[derive(Clone)]
pub struct BuiltProvider {
    /// The constructed provider implementation.
    provider: Arc<dyn Provider>,
    /// Registry key used to build the provider (e.g. "openai", "mock").
    pub key: String,
    /// Model name (e.g. "gpt-4o", "echo").
    pub model: String,
}

impl BuiltProvider {
    /// Create a new `BuiltProvider` from a boxed provider.
    pub fn new(provider: Box<dyn Provider>, key: String, model: String) -> Self {
        Self {
            provider: Arc::from(provider),
            key,
            model,
        }
    }

    /// Wrap a provider implementation.
    #[doc(hidden)]
    pub fn from_provider(provider: Box<dyn Provider>, key: &str, model: &str) -> Self {
        Self {
            provider: Arc::from(provider),
            key: key.to_string(),
            model: model.to_string(),
        }
    }

    /// Returns the registry key used to build this provider.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the model name.
    pub fn model(&self) -> &str {
        &self.model
    }
}

impl std::fmt::Debug for BuiltProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltProvider")
            .field("key", &self.key)
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl Provider for BuiltProvider {
    fn generate(
        &self,
        messages: Vec<ChatMessage>,
    ) -> std::pin::Pin<
        Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>,
    > {
        self.provider.generate(messages)
    }

    fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<serde_json::Value>,
    ) -> std::pin::Pin<
        Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>,
    > {
        self.provider.generate_with_tools(messages, tools)
    }
}

/// Abstract factory for constructing and validating providers.
///
/// Implemented in `runie-provider` so that `runie-core` can avoid a circular
/// dependency on the concrete provider crate. The actor is the sole
/// interactive path that invokes this factory in production.
pub trait ProviderFactory: Send + Sync + 'static {
    /// Build a provider for `provider`/`model` using credentials in `config`.
    fn build(
        &self,
        provider: &str,
        model: &str,
        config: &Config,
    ) -> Result<BuiltProvider, ProviderError>;

    /// Validate `api_key` against `base_url` and return available model IDs.
    fn validate_key(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<String>>> + Send + '_>>;

    /// Resolve the `(base_url, api_key)` pair for `provider` from `config`.
    fn resolve_credentials(&self, provider: &str, config: &Config) -> (String, String);
}
