#![warn(clippy::all)]

//! Runie Provider - Concrete LLM provider implementations

pub mod config;
pub mod factory;
pub mod framing;
pub mod mock;
pub mod openai;
pub mod protocol;
pub mod retry;

pub use config::Config;
pub use factory::DynProviderFactory;
pub use mock::{MockProvider, MockStreamingProvider};
pub use openai::OpenAiProvider;

use anyhow::Result;
use runie_core::actors::provider::BuiltProvider;
use runie_core::provider_event::ProviderEvent;
use runie_core::message::ChatMessage;
use runie_core::provider::{Provider, ProviderError};
use runie_core::provider_registry;

/// Default timeout for API key validation requests.
pub const VALIDATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

// ---------------------------------------------------------------------------
// DynProvider — provider handle with construction helpers
// ---------------------------------------------------------------------------

/// Provider handle that wraps a built provider.
///
/// This type wraps [`BuiltProvider`] and adds construction helpers that depend
/// on `runie-provider` internals. It implements [`Provider`] directly.
#[derive(Clone, Debug)]
pub struct DynProvider {
    inner: BuiltProvider,
}

impl DynProvider {
    /// Build a provider from the saved config file, falling back to environment
    /// variables when the config does not specify a value.
    pub fn new_with_config(
        key: &str,
        model: &str,
        config: &runie_core::config::Config,
    ) -> Result<Self, ProviderError> {
        build_provider(key, model, Some(config)).map(|b| DynProvider { inner: b })
    }

    /// Wrap a built provider.
    pub fn from_built(built: BuiltProvider) -> Self {
        DynProvider { inner: built }
    }

    /// Wrap an arbitrary provider implementation.
    #[doc(hidden)]
    pub fn from_provider(provider: Box<dyn Provider>, key: &str, model: &str) -> Self {
        DynProvider { inner: BuiltProvider::from_provider(provider, key, model) }
    }

    /// Returns the registry key used to build this provider.
    pub fn key(&self) -> &str {
        self.inner.key()
    }

    /// Returns the model name.
    pub fn model(&self) -> &str {
        self.inner.model()
    }
}

impl Provider for DynProvider {
    fn generate(
        &self,
        messages: Vec<ChatMessage>,
    ) -> std::pin::Pin<
        Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>,
    > {
        self.inner.generate(messages)
    }

    fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<serde_json::Value>,
    ) -> std::pin::Pin<
        Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>,
    > {
        self.inner.generate_with_tools(messages, tools)
    }
}

impl From<BuiltProvider> for DynProvider {
    fn from(built: BuiltProvider) -> Self {
        DynProvider { inner: built }
    }
}

impl AsRef<BuiltProvider> for DynProvider {
    fn as_ref(&self) -> &BuiltProvider {
        &self.inner
    }
}

// ---------------------------------------------------------------------------
// Provider construction
// ---------------------------------------------------------------------------

/// Check whether `key` is known in the registry.
pub fn is_known(key: &str) -> bool {
    provider_registry::is_known_provider(key)
}

/// Resolve API key and base URL for a provider.
///
/// When a config is supplied, use the layered resolver
/// (env > dotenv > config file) so keys saved during onboarding are available
/// even when the env var is empty. Otherwise fall back to the env var only.
fn resolve_credentials(
    key: &str,
    meta: &runie_core::provider_registry::ProviderMeta,
    config: Option<&runie_core::config::Config>,
) -> (String, String) {
    let (api_key, base_url) = if let Some(cfg) = config {
        let resolver = config::ProviderConfigResolver::from_config(cfg);
        (
            resolver.resolve_api_key(key).unwrap_or_default(),
            resolver
                .resolve_base_url(key)
                .unwrap_or_else(|| meta.base_url.to_string()),
        )
    } else {
        let api_key = if meta.env_var.is_empty() {
            String::new()
        } else {
            std::env::var(meta.env_var).unwrap_or_default()
        };
        (api_key, meta.base_url.to_string())
    };
    (
        api_key.trim().to_string(),
        base_url.trim_end_matches('/').to_string(),
    )
}

/// Build a provider from a registry key and model name.
///
/// **No silent Mock fallback.** Unknown keys return `Err(UnknownProvider)`.
/// If the API key is not set (and `RUNIE_MOCK` is not enabled), returns
/// `Err(MissingApiKey)`.
pub fn build_provider(
    key: &str,
    model: &str,
    config: Option<&runie_core::config::Config>,
) -> Result<BuiltProvider, ProviderError> {
    if key == "mock" && provider_registry::is_mock_enabled() {
        return Ok(build_mock_provider(key, model));
    }

    let meta = provider_registry::find_provider(key)
        .ok_or_else(|| ProviderError::UnknownProvider(key.to_string()))?;

    let (api_key, base_url) = resolve_credentials(key, meta, config);
    if api_key.is_empty() && !provider_registry::is_mock_enabled() {
        return Err(ProviderError::MissingApiKey(meta.env_var.to_string()));
    }

    let provider = build_openai_provider(api_key, model, &base_url);
    Ok(BuiltProvider::new(provider, key.to_string(), model.to_string()))
}

fn build_mock_provider(key: &str, model: &str) -> BuiltProvider {
    let provider: Box<dyn Provider> = if std::env::var_os("RUNIE_MOCK_DELAY").is_some() {
        Box::new(MockProvider::with_delay(300, 800))
    } else {
        Box::new(MockProvider::default())
    };
    BuiltProvider::new(provider, key.to_string(), model.to_string())
}

fn build_openai_provider(api_key: String, model: &str, base_url: &str) -> Box<dyn Provider> {
    let p = OpenAiProvider::new(api_key, model).with_base_url(base_url);
    let p = if let Some(meta) = provider_registry::find_model(model) {
        p.with_model_meta(meta)
    } else {
        p
    };
    Box::new(retry::RetryProvider::new(p))
}

// ---------------------------------------------------------------------------
// Provider construction helpers
// ---------------------------------------------------------------------------

/// Try each provider in the chain until one builds successfully, using the
/// provided config to resolve API keys and base URLs.
pub fn build_provider_with_fallback(
    chain: &[&str],
    model: &str,
    config: &runie_core::config::Config,
) -> Result<BuiltProvider, ProviderError> {
    let mut last_err = None;
    for key in chain {
        match build_provider(key, model, Some(config)) {
            Ok(provider) => return Ok(provider),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| ProviderError::UnknownProvider("none".to_string())))
}

// ---------------------------------------------------------------------------
// API key validation
// ---------------------------------------------------------------------------

/// Validate an API key by calling the provider's `/models` endpoint.
/// Returns a list of available model IDs on success.
///
/// Fails after [`VALIDATION_TIMEOUT`] so the UI never gets stuck waiting
/// for an unreachable or unresponsive provider.
pub async fn validate_api_key(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    validate_api_key_with_timeout(base_url, api_key, VALIDATION_TIMEOUT).await
}

/// Validate an API key with a configurable request timeout.
pub async fn validate_api_key_with_timeout(
    base_url: &str,
    api_key: &str,
    timeout: std::time::Duration,
) -> Result<Vec<String>> {
    let fut = fetch_models(base_url, api_key, timeout);
    match tokio::time::timeout(timeout, fut).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!("API validation timed out after {}s", timeout.as_secs()),
    }
}

async fn fetch_models(
    base_url: &str,
    api_key: &str,
    timeout: std::time::Duration,
) -> Result<Vec<String>> {
    let client = build_http_client(timeout)?;
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("API validation failed: {}", text);
    }

    let json: serde_json::Value = resp.json().await?;
    Ok(extract_model_ids(&json))
}

fn build_http_client(timeout: std::time::Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .build()?)
}

fn extract_model_ids(json: &serde_json::Value) -> Vec<String> {
    json.get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Re-exports for consumers
// ---------------------------------------------------------------------------

/// Re-export so `runie_agent` can use it without a deep dependency.
pub use runie_core::provider::ProviderError as UnknownProviderError;

/// Spawn a production `HeadlessRuntime` using the default provider factory.
///
/// This is the shared entry point for all non-interactive binaries so they do
/// not duplicate the runtime setup.
pub async fn spawn_headless_runtime() -> runie_core::headless_runtime::HeadlessRuntime {
    use runie_core::bus::EventBus;
    use runie_core::event::Event;
    use std::sync::Arc;

    runie_core::headless_runtime::HeadlessRuntime::spawn(
        EventBus::<Event>::new(10),
        Arc::new(DynProviderFactory),
    )
    .await
    .expect("config must load")
}

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod tests;
