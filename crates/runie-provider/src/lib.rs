#![warn(clippy::all)]

//! Runie Provider - Concrete LLM provider implementations

pub mod config;
pub mod factory;
pub mod http;
pub mod model_client;
pub mod protocol;
pub mod retry;

#[cfg(feature = "openai")]
pub mod openai;

#[cfg(feature = "mock")]
pub mod mock;

use crate::retry::{is_retryable, with_retry};

// ---------------------------------------------------------------------------
// Re-exports from runie-core
// ---------------------------------------------------------------------------

// Provider trait and registry (moved to runie-core for cross-crate access).
pub use runie_core::provider::registry::{
    display_name, find_model, find_provider, find_provider_by_env_var, is_known_provider,
    is_mock_enabled, known_providers, ModelMeta, ModelMetaBuilder, ProviderMeta, ProviderMetaBuilder,
};
pub use runie_core::provider::ProviderError;
pub use runie_core::provider::{Provider, ProviderMetadata, RetryConfig, ResponseChunk};

// Model catalog types.
pub use runie_core::model_catalog::configured::configured_models_catalog;
pub use runie_core::model_catalog::{filter_models, model_catalog, ModelCapabilities, ModelInfo};

pub use runie_core::config::Config;
pub use factory::BuiltProviderFactory;
pub use model_client::{ModelClient, TurnSession};

#[cfg(feature = "openai")]
pub use openai::OpenAiProvider;

#[cfg(feature = "mock")]
pub use mock::{MockProvider, MockStreamingProvider};
pub use runie_core::proto::ProviderConfig;
use std::sync::Arc;

use anyhow::Result;

/// Default timeout for API key validation requests.
pub const VALIDATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

// Re-export HTTP timeout constants from runie-core so all provider crates share the same values.
pub use runie_core::provider::{REQUEST_TIMEOUT, CONNECT_TIMEOUT};

/// Re-export `BuiltProvider` from `runie-core`.
pub use runie_core::actors::provider::BuiltProvider;

// ---------------------------------------------------------------------------
// Provider construction helpers
// ---------------------------------------------------------------------------

/// Check whether `key` is known in the registry.
pub fn is_known(key: &str) -> bool {
    is_known_provider(key)
}

/// Resolve API key and base URL for a provider.
fn resolve_credentials(
    key: &str,
    meta: &ProviderMeta,
    config: Option<Arc<dyn ProviderConfig>>,
) -> (String, String) {
    let (api_key, base_url) = if let Some(cfg) = config {
        let resolver = config::ProviderConfigResolver::new(cfg);
        (
            resolver.resolve_api_key(key).unwrap_or_default(),
            resolver
                .resolve_base_url(key)
                .unwrap_or_else(|| meta.base_url.to_owned()),
        )
    } else {
        // When no config is provided, use CredentialResolver for unified priority:
        // env var → dotenv → keyring → config
        let resolver = runie_core::auth::CredentialResolver::new();
        let api_key = resolver
            .resolve_api_key(key)
            .unwrap_or_default();
        (api_key, meta.base_url.to_owned())
    };
    (
        http::normalize_api_key(&api_key),
        http::normalize_base_url(&base_url),
    )
}

/// Build a provider from a registry key and model name.
pub fn build_provider(
    key: &str,
    model: &str,
    config: Option<Arc<dyn ProviderConfig>>,
) -> Result<BuiltProvider, ProviderError> {
    #[cfg(feature = "mock")]
    {
        if key == "mock" && is_mock_enabled() {
            return Ok(build_mock_provider(key, model));
        }
    }
    #[cfg(not(feature = "mock"))
    ]
    {
        if key == "mock" {
            return Err(ProviderError::UnknownProvider(key.to_owned()));
        }
    }

    let meta = find_provider(key).ok_or_else(|| ProviderError::UnknownProvider(key.to_owned()))?;

    let (api_key, base_url) = resolve_credentials(key, &meta, config);
    if api_key.is_empty() && !is_mock_enabled() {
        return Err(ProviderError::MissingApiKey(meta.env_var.to_owned().into()));
    }

    #[cfg(feature = "openai")]
    {
        let provider = build_openai_provider(api_key, model, &base_url);
        Ok(BuiltProvider::new(provider, key.to_owned(), model.to_owned()))
    }
    #[cfg(not(feature = "openai"))]
    {
        let _ = (api_key, model, base_url);
        Err(ProviderError::UnknownProvider(key.to_owned()))
    }
}

#[cfg(feature = "mock")]
fn build_mock_provider(key: &str, model: &str) -> BuiltProvider {
    let provider: Box<dyn Provider> = if std::env::var_os("RUNIE_MOCK_DELAY").is_some() {
        // Use small delay (5-10ms) for fast deterministic tests
        Box::new(MockProvider::with_delay(5, 10))
    } else {
        Box::new(MockProvider::default())
    };
    BuiltProvider::new(provider, key.to_owned(), model.to_owned())
}

#[cfg(feature = "openai")]
fn build_openai_provider(api_key: String, model: &str, base_url: &str) -> Box<dyn Provider> {
    // Use the cached HTTP client so TCP connections are reused across turns.
    let client = BuiltProvider::cached_http_client("openai", base_url);
    let p = OpenAiProvider::from_http_client(client, api_key, model).with_base_url(base_url);
    let p = if let Some(meta) = find_model(model) {
        p.with_model_meta(meta)
    } else {
        p
    };
    // Retries are handled by backon for stream establishment (see stream.rs)
    Box::new(p)
}

/// Build a provider from the saved config file, falling back to environment
/// variables when the config does not specify a value.
pub fn build_provider_with_config(
    key: &str,
    model: &str,
    config: &runie_core::config::Config,
) -> Result<BuiltProvider, ProviderError> {
    build_provider(key, model, Some(Arc::new(config.clone()) as Arc<dyn ProviderConfig>))
}

/// Wrap an arbitrary provider implementation.
pub fn build_provider_from_boxed(
    provider: Box<dyn Provider>,
    key: &str,
    model: &str,
) -> BuiltProvider {
    BuiltProvider::from_provider(provider, key, model)
}

/// Try each provider until one builds successfully.
pub fn build_provider_with_fallback(
    chain: &[&str],
    model: &str,
    config: Arc<dyn ProviderConfig>,
) -> Result<BuiltProvider, ProviderError> {
    let mut last_err = None;
    for key in chain {
        match build_provider(key, model, Some(config.clone())) {
            Ok(provider) => return Ok(provider),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| ProviderError::UnknownProvider("none".to_owned())))
}

// ---------------------------------------------------------------------------
// API key validation
// ---------------------------------------------------------------------------

pub async fn validate_api_key(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    validate_api_key_with_timeout(base_url, api_key, VALIDATION_TIMEOUT).await
}

pub async fn validate_api_key_with_timeout(
    base_url: &str,
    api_key: &str,
    timeout: std::time::Duration,
) -> Result<Vec<String>> {
    // Apply retry via backon to transient errors, bounded by overall timeout.
    match tokio::time::timeout(
        timeout,
        with_retry(|| async { fetch_models(base_url, api_key, timeout).await }),
    )
    .await
    {
        Ok(Ok(models)) => Ok(models),
        Ok(Err(e)) => {
            if is_retryable(&e) {
                anyhow::bail!("API validation failed after retries: {e}");
            }
            Err(e)
        }
        Err(_) => anyhow::bail!("API validation timed out after {}s", timeout.as_secs()),
    }
}

async fn fetch_models(
    base_url: &str,
    api_key: &str,
    timeout: std::time::Duration,
) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .build()?;
    let url = http::request_url(base_url, "models");
    let resp = client
        .get(&url)
        .header("Authorization", http::bearer_header(api_key))
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("API validation failed: {}", text);
    }

    let json: serde_json::Value = resp.json().await?;
    Ok(json
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Re-exports for consumers
// ---------------------------------------------------------------------------

pub async fn spawn_headless_runtime(
) -> anyhow::Result<runie_core::headless_runtime::HeadlessRuntime> {
    use runie_core::bus::EventBus;
    use runie_core::event::Event;
    use std::sync::Arc;

    runie_core::headless_runtime::HeadlessRuntime::spawn(
        EventBus::<Event>::new(10),
        Arc::new(BuiltProviderFactory::new()),
    )
    .await
}

#[cfg(test)]
mod tests;
