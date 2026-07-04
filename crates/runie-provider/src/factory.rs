//! Concrete [`ProviderFactory`] implementation backed by `BuiltProvider`.

use async_trait::async_trait;
use secrecy::ExposeSecret;
use std::sync::Arc;

use crate::config::ProviderConfigResolver;
use crate::{build_provider, find_provider, validate_api_key, ProviderError};
use runie_core::actors::provider::{BuiltProvider, ProviderFactory};
use runie_core::auth::KeyringStore;
use runie_core::config::Config;
use runie_core::proto::ProviderConfig;

#[cfg(feature = "replay")]
use crate::replay::{Protocol, ReplayProvider};

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
        Self {
            keyring_store: None,
        }
    }

    /// Create a factory with an injectable keyring store.
    ///
    /// Use this in tests to avoid hitting the OS keyring.
    pub fn with_keyring_store(store: Arc<dyn KeyringStore>) -> Self {
        Self {
            keyring_store: Some(store),
        }
    }

    /// Try to build a replay provider from `RUNIE_REPLAY_FIXTURES` env var.
    ///
    /// The env var should contain a comma-separated list of file paths to SSE
    /// fixtures. The protocol is inferred from the fixture contents, or can be
    /// explicitly set via `RUNIE_REPLAY_PROTOCOL` (values: `openai`, `anthropic`).
    #[cfg(feature = "replay")]
    fn try_build_replay_provider(provider: &str, model: &str) -> Option<BuiltProvider> {
        let fixture_list = std::env::var("RUNIE_REPLAY_FIXTURES").ok()?;
        if fixture_list.trim().is_empty() {
            return None;
        }

        let paths: Vec<&str> = fixture_list.split(',').map(str::trim).collect();
        let mut fixtures = Vec::new();

        for path in paths {
            if path.is_empty() {
                continue;
            }
            match std::fs::read_to_string(path) {
                Ok(contents) => fixtures.push(contents),
                Err(e) => {
                    tracing::warn!(path, error = %e, "failed to read replay fixture");
                    return None;
                }
            }
        }

        if fixtures.is_empty() {
            return None;
        }

        // Determine protocol from env var or fixture inference.
        let protocol = match std::env::var("RUNIE_REPLAY_PROTOCOL")
            .ok()
            .as_deref()
        {
            Some("anthropic") => Protocol::Anthropic,
            Some("openai") => Protocol::OpenAi,
            _ => ReplayProvider::infer_protocol(&fixtures),
        };

        let replay = ReplayProvider::new(fixtures, protocol);
        tracing::debug!(provider, model, protocol = ?protocol, "using replay provider");

        // Use provided provider/model, or defaults for replay context.
        let key = if provider.is_empty() || provider == "replay" {
            "openai"
        } else {
            provider
        };
        let model = if model.is_empty() { "replay" } else { model };

        Some(BuiltProvider::from_provider(
            Box::new(replay),
            key,
            model,
        ))
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
        // Check for replay mode first.
        #[cfg(feature = "replay")]
        if let Some(replay_provider) = Self::try_build_replay_provider(provider, model) {
            return Ok(replay_provider);
        }

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
        // Expose secret at the boundary where credentials are needed
        let api_key = resolver
            .resolve_api_key(provider)
            .map(|s| s.expose_secret().clone())
            .unwrap_or_default();
        (base_url, api_key)
    }
}

fn default_base_url(provider: &str) -> Option<String> {
    find_provider(provider).map(|m| m.base_url.to_owned())
}
