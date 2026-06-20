//! Concrete [`ProviderFactory`] implementation backed by `DynProvider`.

use std::future::Future;
use std::pin::Pin;

use runie_core::actors::provider::{BuiltProvider, ProviderFactory};
use runie_core::config::Config;
use runie_core::provider::ProviderError;
use runie_core::provider_registry;

use crate::{validate_api_key, DynProvider};

/// The production provider factory.
///
/// This is the only production implementation of [`ProviderFactory`] and the
/// only production code path that calls `DynProvider::new_with_config`.
pub struct DynProviderFactory;

impl ProviderFactory for DynProviderFactory {
    fn build(
        &self,
        provider: &str,
        model: &str,
        config: &Config,
    ) -> Result<BuiltProvider, ProviderError> {
        let dyn_provider = DynProvider::new_with_config(provider, model, config)?;
        let key = dyn_provider.key().to_string();
        let model = dyn_provider.model().to_string();
        Ok(BuiltProvider {
            provider: Box::new(dyn_provider),
            key,
            model,
        })
    }

    fn validate_key(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<String>>> + Send + '_>> {
        let base_url = base_url.to_string();
        let api_key = api_key.to_string();
        Box::pin(async move { validate_api_key(&base_url, &api_key).await })
    }

    fn resolve_credentials(&self, provider: &str, config: &Config) -> (String, String) {
        let resolver = crate::config::ProviderConfigResolver::from_config(config);
        let base_url = resolver
            .resolve_base_url(provider)
            .or_else(|| default_base_url(provider))
            .unwrap_or_default();
        let api_key = resolver.resolve_api_key(provider).unwrap_or_default();
        (base_url, api_key)
    }
}

fn default_base_url(provider: &str) -> Option<String> {
    provider_registry::find_provider(provider).map(|m| m.base_url.to_string())
}
