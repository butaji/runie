//! Abstract factory used by `ProviderActor` to build and validate providers.

use std::future::Future;
use std::pin::Pin;

use crate::config::Config;
use crate::provider::{Provider, ProviderError};

/// Result of building a provider through the actor.
pub struct BuiltProvider {
    /// The constructed provider implementation.
    pub provider: Box<dyn Provider>,
    /// Registry key used to build the provider (e.g. "openai", "mock").
    pub key: String,
    /// Model name (e.g. "gpt-4o", "echo").
    pub model: String,
}

impl std::fmt::Debug for BuiltProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltProvider")
            .field("key", &self.key)
            .field("model", &self.model)
            .finish_non_exhaustive()
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
