//! Concrete [`ProviderFactory`] implementation backed by `BuiltProvider`.

use async_trait::async_trait;
use std::sync::Arc;

use crate::config::ProviderConfigResolver;
use crate::{build_provider, find_provider, validate_api_key, ProviderError};
use runie_core::actors::provider::{BuiltProvider, ProviderFactory};
use runie_core::auth::KeyringStore;
use runie_core::config::Config;
use runie_core::proto::ProviderConfig;

/// The production provider factory.
///
/// This is the only production implementation of [`ProviderFactory`] and the
/// only production code path that constructs providers.
#[derive(Clone)]
pub struct BuiltProviderFactory {
    /// Optional keyring store for credential resolution.
    /// When `None`, uses `OsKeyringStore` (production default).
    keyring_store: Option<Arc<dyn KeyringStore>>,
}

impl Default for BuiltProviderFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltProviderFactory {
    /// Create a new factory using the OS keyring.
    pub fn new() -> Self {
        Self { keyring_store: None }
    }

    /// Create a factory with an injectable keyring store.
    ///
    /// Use this in tests to avoid hitting the OS keyring.
    pub fn with_keyring_store(store: Arc<dyn KeyringStore>) -> Self {
        Self {
            keyring_store: Some(store),
        }
    }
}

#[async_trait]
impl ProviderFactory for BuiltProviderFactory {
    fn build(
        &self,
        provider: &str,
        model: &str,
        config: &Config,
    ) -> Result<BuiltProvider, ProviderError> {
        build_provider(
            provider,
            model,
            Some(Arc::new(config.clone()) as Arc<dyn ProviderConfig>),
        )
    }

    async fn validate_key(&self, base_url: &str, api_key: &str) -> anyhow::Result<Vec<String>> {
        validate_api_key(base_url, api_key).await
    }

    fn resolve_credentials(&self, provider: &str, config: &Config) -> (String, String) {
        let config_arc = Arc::new(config.clone()) as Arc<dyn ProviderConfig>;
        let resolver = if let Some(store) = &self.keyring_store {
            ProviderConfigResolver::with_keyring_store(config_arc, store.clone())
        } else {
            ProviderConfigResolver::new(config_arc)
        };
        let base_url = resolver
            .resolve_base_url(provider)
            .or_else(|| default_base_url(provider))
            .unwrap_or_default();
        let api_key = resolver.resolve_api_key(provider).unwrap_or_default();
        (base_url, api_key)
    }
}

fn default_base_url(provider: &str) -> Option<String> {
    find_provider(provider).map(|m| m.base_url.to_owned())
}
