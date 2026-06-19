#![warn(clippy::all)]

//! Runie Provider - Concrete LLM provider implementations

pub mod config;
pub mod mock;
pub mod openai;
pub mod planner;
pub mod retry;

pub use config::Config;
pub use mock::{MockProvider, MockStreamingProvider};
pub use openai::OpenAiProvider;

use anyhow::Result;
use runie_core::message::ChatMessage;
use runie_core::provider::{Provider, ProviderError};
use runie_core::provider_registry;
use std::pin::Pin;

/// Default timeout for API key validation requests.
pub const VALIDATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

// ---------------------------------------------------------------------------
// DynProvider — dynamic dispatch wrapper
// ---------------------------------------------------------------------------

/// A provider owned behind a trait object, enabling dynamic dispatch.
///
/// All concrete providers (`OpenAiProvider`, `MockProvider`, etc.) implement
/// `Provider` and can be wrapped here.
pub struct DynProvider {
    inner: Box<dyn Provider>,
    /// The registry key used to build this provider (e.g. "openai", "mock").
    key: String,
    /// The model name (e.g. "gpt-4o", "echo").
    model: String,
}

impl DynProvider {
    /// Build a provider by registry key and model name.
    ///
    /// Returns `Err(UnknownProvider)` for unknown keys (no silent Mock fallback).
    /// Returns `Err(MissingApiKey)` when `RUNIE_MOCK` is not set and the key
    /// requires an API key.
    ///
    /// This variant only checks environment variables. Use [`Self::new_with_config`]
    /// to also read the API key/base URL from the saved config file.
    pub fn new(key: &str, model: &str) -> Result<Self, ProviderError> {
        build_dyn_provider(key, model, None)
    }

    /// Build a provider from the saved config file, falling back to environment
    /// variables when the config does not specify a value.
    pub fn new_with_config(
        key: &str,
        model: &str,
        config: &runie_core::config::Config,
    ) -> Result<Self, ProviderError> {
        build_dyn_provider(key, model, Some(config))
    }

    /// Build a provider, returning the key even on error for better error messages.
    pub fn new_checked(key: &str, model: &str) -> Result<Self, ProviderError> {
        build_dyn_provider(key, model, None)
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

impl std::fmt::Debug for DynProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynProvider")
            .field("key", &self.key)
            .field("model", &self.model)
            .finish()
    }
}

impl Provider for DynProvider {
    fn generate(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Pin<
        Box<
            dyn futures::Stream<Item = anyhow::Result<runie_core::llm_event::LLMEvent>> + Send + '_,
        >,
    > {
        self.inner.generate(messages)
    }

    fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<serde_json::Value>,
    ) -> Pin<
        Box<
            dyn futures::Stream<Item = anyhow::Result<runie_core::llm_event::LLMEvent>> + Send + '_,
        >,
    > {
        self.inner.generate_with_tools(messages, tools)
    }
}

// ---------------------------------------------------------------------------
// Provider construction
// ---------------------------------------------------------------------------

/// Check whether `key` is known in the registry.
pub fn is_known(key: &str) -> bool {
    provider_registry::is_known_provider(key)
}

/// Check whether `key` is an OpenAI-compatible provider.
pub fn is_openai_compatible(key: &str) -> bool {
    provider_registry::find_provider(key).is_some()
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

/// Build a `DynProvider` from a registry key and model name.
///
/// **No silent Mock fallback.** Unknown keys return `Err(UnknownProvider)`.
/// If the API key is not set (and `RUNIE_MOCK` is not enabled), returns
/// `Err(MissingApiKey)`.
fn build_dyn_provider(
    key: &str,
    model: &str,
    config: Option<&runie_core::config::Config>,
) -> Result<DynProvider, ProviderError> {
    if key == "mock" && provider_registry::is_mock_enabled() {
        return Ok(build_mock_provider(key, model));
    }

    let meta = provider_registry::find_provider(key)
        .ok_or_else(|| ProviderError::UnknownProvider(key.to_string()))?;

    let (api_key, base_url) = resolve_credentials(key, meta, config);
    if api_key.is_empty() && !provider_registry::is_mock_enabled() {
        return Err(ProviderError::MissingApiKey(meta.env_var.to_string()));
    }

    Ok(build_openai_provider(key, model, api_key, base_url))
}

fn build_mock_provider(key: &str, model: &str) -> DynProvider {
    let provider: Box<dyn Provider> = if std::env::var_os("RUNIE_MOCK_DELAY").is_some() {
        Box::new(MockProvider::with_delay(300, 800))
    } else {
        Box::new(MockProvider::default())
    };
    DynProvider {
        inner: provider,
        key: key.to_string(),
        model: model.to_string(),
    }
}

fn build_openai_provider(
    key: &str,
    model: &str,
    api_key: String,
    base_url: String,
) -> DynProvider {
    let p = OpenAiProvider::new(api_key, model).with_base_url(&base_url);
    let p = if let Some(meta) = provider_registry::find_model(model) {
        p.with_model_meta(meta)
    } else {
        p
    };
    DynProvider {
        inner: Box::new(retry::RetryProvider::new(p)),
        key: key.to_string(),
        model: model.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Legacy helpers (kept for API compatibility during migration)
// ---------------------------------------------------------------------------

/// Build a provider. Returns `DynProvider` or error — no silent fallback.
pub fn build_provider(provider: &str, model: &str) -> DynProvider {
    build_provider_with_warning(provider, model)
        .expect("build_provider_with_warning returns Ok or panic — use new() for explicit errors")
}

/// Build a provider and return a warning message if a known non-critical condition occurred.
/// In the new design there is no warning — the error is returned explicitly.
///
/// This variant only checks environment variables. Use
/// [`build_provider_with_warning_with_config`] to read saved config.
pub fn build_provider_with_warning(
    provider: &str,
    model: &str,
) -> Result<DynProvider, ProviderError> {
    build_dyn_provider(provider, model, None)
}

/// Build a provider using the saved config file.
pub fn build_provider_with_warning_with_config(
    provider: &str,
    model: &str,
    config: &runie_core::config::Config,
) -> Result<DynProvider, ProviderError> {
    build_dyn_provider(provider, model, Some(config))
}

/// Build a provider from `Config`.
pub fn from_config(config: &Config, model: &str) -> DynProvider {
    let chain = config.provider_chain();
    build_provider_with_fallback(&chain, model, config).expect(
        "from_config: provider key is always known or panic — use new() for explicit errors",
    )
}

/// Try each provider in the chain until one builds successfully, using the
/// provided config to resolve API keys and base URLs.
pub fn build_provider_with_fallback(
    chain: &[&str],
    model: &str,
    config: &runie_core::config::Config,
) -> Result<DynProvider, ProviderError> {
    let mut last_err = None;
    for key in chain {
        match build_dyn_provider(key, model, Some(config)) {
            Ok(provider) => return Ok(provider),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| ProviderError::UnknownProvider("none".to_string())))
}

/// Switch a live provider to a new key/model pair, reading credentials from
/// the saved config file.
pub fn switch_provider(
    provider: &mut DynProvider,
    key: &str,
    model: &str,
) -> Result<(), ProviderError> {
    let config = runie_core::config::Config::load(None);
    *provider = build_dyn_provider(key, model, Some(&config))?;
    Ok(())
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

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod tests;
