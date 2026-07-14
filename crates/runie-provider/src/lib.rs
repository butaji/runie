#![warn(clippy::all)]

//! Runie Provider - Concrete LLM provider implementations

use secrecy::ExposeSecret;
use tracing::Instrument;

pub mod config;
pub mod factory;
pub mod http;

pub mod protocol;
pub mod retry;

#[cfg(feature = "openai")]
pub mod openai;

pub mod anthropic;

#[cfg(feature = "mock")]
pub mod mock;

#[cfg(feature = "replay")]
pub mod replay;

#[cfg(feature = "replay")]
pub use replay::{
    compute_replay_key, ReplayKeyBuilder, ReplayProtocol, ReplayProvider, ToolCallEntry,
};

use crate::retry::{is_retryable, with_retry};

// ---------------------------------------------------------------------------
// Re-exports from runie-core
// ---------------------------------------------------------------------------

// Provider trait and registry (moved to runie-core for cross-crate access).
pub use runie_core::provider::registry::{
    display_name, find_model, find_model_for_provider, find_provider, find_provider_by_env_var,
    is_known_provider, is_mock_enabled, known_providers, strip_provider_prefix, ModelMeta,
    ModelMetaBuilder, ProviderMeta, ProviderMetaBuilder,
};
pub use runie_core::provider::ProviderError;
pub use runie_core::provider::{Provider, ProviderMetadata, ResponseChunk, RetryConfig};

// Model catalog types.
pub use runie_core::model_catalog::configured::configured_models_catalog;
pub use runie_core::model_catalog::{filter_models, model_catalog, ModelCapabilities, ModelInfo};

pub use factory::BuiltProviderFactory;
pub use runie_core::config::Config;

#[cfg(feature = "openai")]
pub use openai::OpenAiProvider;

#[cfg(feature = "mock")]
pub use mock::{MockProvider, MockProviderBuilder, MockStreamingProvider};
pub use runie_core::proto::ProviderConfig;
use std::sync::Arc;

use anyhow::Result;

/// Default timeout for API key validation requests.
pub const VALIDATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

// Re-export HTTP timeout constants from runie-core so all provider crates share the same values.
pub use runie_core::provider::{CONNECT_TIMEOUT, REQUEST_TIMEOUT};

/// Minimum delay for mock provider (milliseconds).
#[cfg(feature = "mock")]
pub const MOCK_DELAY_MIN_MS: u64 = 5;

/// Maximum delay for mock provider (milliseconds).
#[cfg(feature = "mock")]
pub const MOCK_DELAY_MAX_MS: u64 = 10;

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
///
/// Returns `(api_key, base_url)` where api_key is a `SecretString`.
fn resolve_credentials(
    key: &str,
    meta: &ProviderMeta,
    config: Option<Arc<dyn ProviderConfig>>,
) -> (secrecy::SecretString, String) {
    let (api_key, base_url) = if let Some(cfg) = config {
        let resolver = config::ProviderConfigResolver::new(cfg);
        (
            resolver.resolve_api_key(key),
            resolver
                .resolve_base_url(key)
                .unwrap_or_else(|| meta.base_url.to_owned()),
        )
    } else {
        // When no config is provided, use CredentialResolver for unified priority:
        // env var → dotenv → keyring → config
        let resolver = runie_core::auth::CredentialResolver::new();
        let api_key = resolver.resolve_api_key(key);
        (api_key, meta.base_url.to_owned())
    };
    (
        api_key.unwrap_or_else(|| secrecy::SecretString::from(String::new())),
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
    #[cfg(not(feature = "mock"))]
    {
        if key == "mock" {
            return Err(ProviderError::UnknownProvider(key.to_owned()));
        }
    }

    let meta = find_provider(key).ok_or_else(|| ProviderError::UnknownProvider(key.to_owned()))?;

    let (api_key, base_url) = resolve_credentials(key, &meta, config);
    if api_key.expose_secret().is_empty() && !is_mock_enabled() {
        return Err(ProviderError::MissingApiKey(meta.env_var.to_owned().into()));
    }

    // Model names in config may be provider-prefixed ("openai/gpt-4o"). Strip the
    // prefix when it matches the provider key so the API receives the bare model
    // name it expects, while still looking up metadata from the right provider.
    let bare_model = strip_provider_prefix(key, model);

    #[cfg(feature = "openai")]
    {
        let provider = build_openai_provider(api_key, bare_model, &base_url, key, model);
        Ok(BuiltProvider::new(
            provider,
            key.to_owned(),
            model.to_owned(),
        ))
    }
    #[cfg(not(feature = "openai"))]
    {
        let _ = (api_key, model, base_url);
        Err(ProviderError::UnknownProvider(key.to_owned()))
    }
}

#[cfg(feature = "mock")]
fn build_mock_provider(key: &str, model: &str) -> BuiltProvider {
    use crate::mock::MockProviderBuilder;

    let base = if std::env::var_os("RUNIE_MOCK_DELAY").is_some() {
        MockProviderBuilder::new().with_delay(MOCK_DELAY_MIN_MS, MOCK_DELAY_MAX_MS)
    } else {
        MockProviderBuilder::new()
    };
    let provider: Box<dyn Provider> = match model {
        "list_dir" => Box::new(base.list_dir().build()),
        "read_file" => Box::new(base.read_file().build()),
        "write_file" => Box::new(base.write_file().build()),
        "edit_file" => Box::new(base.edit_file().build()),
        "bash" => Box::new(base.bash().build()),
        "grep" => Box::new(base.grep().build()),
        "find" => Box::new(base.find().build()),
        "malformed" => Box::new(base.malformed().build()),
        "markup" => Box::new(base.markup().build()),
        _ => Box::new(base.build()),
    };
    BuiltProvider::new(provider, key.to_owned(), model.to_owned())
}

#[cfg(feature = "openai")]
fn build_openai_provider(
    api_key: secrecy::SecretString,
    model: &str,
    base_url: &str,
    provider_key: &str,
    original_model: &str,
) -> Box<dyn Provider> {
    // Use the cached HTTP client so TCP connections are reused across turns.
    let client = BuiltProvider::cached_http_client("openai", base_url);
    // Convert SecretString to String for OpenAiProvider (normalization happens inside)
    let api_key_str = api_key.expose_secret().to_string();
    let p = OpenAiProvider::from_http_client(client, api_key_str, model).with_base_url(base_url);
    // Look up model metadata from the intended provider. The original model name
    // may be provider-prefixed ("openai/gpt-4o"); find_model_for_provider strips
    // the prefix when it matches the provider key.
    let p = if let Some(meta) = find_model_for_provider(provider_key, original_model) {
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
    build_provider(
        key,
        model,
        Some(Arc::new(config.clone()) as Arc<dyn ProviderConfig>),
    )
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
    let span = tracing::info_span!("validate_api_key", base_url = %base_url);
    async move {
        tracing::debug!("validating API key");
        // Apply retry via backon to transient errors, bounded by overall timeout.
        match tokio::time::timeout(
            timeout,
            with_retry(|| async { fetch_models(base_url, api_key, timeout).await }),
        )
        .await
        {
            Ok(Ok(models)) => {
                tracing::debug!(model_count = %models.len(), "API key validated successfully");
                Ok(models)
            }
            Ok(Err(e)) => {
                if is_retryable(&e) {
                    tracing::warn!(error = %e, "API validation failed after retries");
                    anyhow::bail!("API validation failed after retries: {e}");
                }
                tracing::warn!(error = %e, "API validation failed");
                Err(e)
            }
            Err(_) => {
                tracing::warn!(timeout_secs = %timeout.as_secs(), "API validation timed out");
                anyhow::bail!("API validation timed out after {}s", timeout.as_secs())
            }
        }
    }
    .instrument(span)
    .await
}

async fn fetch_models(
    base_url: &str,
    api_key: &str,
    timeout: std::time::Duration,
) -> Result<Vec<String>> {
    let span = tracing::debug_span!("fetch_models", base_url = %base_url);
    async move {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(timeout)
            .build()?;
        let url = http::request_url(base_url, "models");
        tracing::trace!("fetching models from {}", url);
        let resp = client
            .get(&url)
            .header("Authorization", http::bearer_header(api_key))
            .send()
            .await?;

        let status = resp.status();
        tracing::trace!(status = %status, "received response");

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!(status = %status, "API request failed");
            let summary = sanitize_provider_error(status, &text);
            anyhow::bail!("API validation failed: {}", summary);
        }
        let json: serde_json::Value = resp.json().await?;
        Ok(json
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                    // Gemini's OpenAI-compatible /models endpoint prefixes ids
                    // with "models/"; the registry stores bare names.
                    .map(|id| id.strip_prefix("models/").unwrap_or(&id).to_owned())
                    .collect()
            })
            .unwrap_or_default())
    }
    .instrument(span)
    .await
}

/// Extract a short, user-readable error summary from a provider error response.
/// Avoids dumping raw JSON into the TUI transient message area.
fn sanitize_provider_error(status: reqwest::StatusCode, body: &str) -> String {
    // Try common provider JSON shapes first.
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        // Anthropic / OpenAI style: { "error": { "message": "..." } }
        if let Some(msg) = json
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
        {
            return msg.to_owned();
        }
        // Some providers return { "message": "..." }
        if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
            return msg.to_owned();
        }
    }
    // Fall back to a concise status-based message.
    match status.as_u16() {
        401 => "Invalid API key (unauthorized).".to_owned(),
        403 => "API key does not have permission for this request.".to_owned(),
        429 => "Rate limited. Please wait a moment and try again.".to_owned(),
        500..=599 => "Provider server error. Please try again later.".to_owned(),
        _ => format!("HTTP {}", status),
    }
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
