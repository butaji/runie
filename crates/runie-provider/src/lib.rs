#![warn(clippy::all)]

//! Runie Provider - Concrete LLM provider implementations

pub mod config;
pub mod mock;
pub mod model;
pub mod openai;

pub use config::Config;
pub use mock::{MockProvider, MockStreamingProvider};
pub use model::{ModelId, ModelRegistry};
pub use openai::OpenAiProvider;

use anyhow::Result;
use runie_core::provider::{Message, Provider, ResponseChunk};

/// Runtime provider selection — closed enum for static dispatch.
pub enum AnyProvider {
    Mock(MockProvider),
    OpenAi(OpenAiProvider),
}

impl AnyProvider {
    fn build(provider: &str, model: &str) -> (Self, Option<String>) {
        let config = Config::load();
        Self::build_with_config(provider, model, &config)
    }

    fn build_with_config(provider: &str, model: &str, config: &Config) -> (Self, Option<String>) {
        let resolver = crate::config::ProviderConfigResolver::from_config(config);

        if let Some(api_key) = resolver.resolve_api_key(provider) {
            if !api_key.is_empty() {
                let base_url = resolver.resolve_base_url(provider);
                let mut p = OpenAiProvider::new(api_key, model);
                if let Some(url) = base_url {
                    p = p.with_base_url(url);
                }
                return (Self::OpenAi(p), None);
            }
        }

        match provider {
            "openai" => {
                let warning = "OPENAI_API_KEY not set, falling back to mock".to_string();
                let mock = if std::env::var("RUNIE_MOCK_DELAY").is_ok() {
                    MockProvider::with_delay(500, 3000)
                } else {
                    MockProvider::default()
                };
                (Self::Mock(mock), Some(warning))
            }
            _ => {
                if std::env::var("RUNIE_MOCK_DELAY").is_ok() {
                    (Self::Mock(MockProvider::with_delay(500, 3000)), None)
                } else {
                    (Self::Mock(MockProvider::default()), None)
                }
            }
        }
    }

    pub fn new(provider: &str, model: &str) -> Self {
        Self::build(provider, model).0
    }

    pub fn new_with_warning(provider: &str, model: &str) -> (Self, Option<String>) {
        Self::build(provider, model)
    }

    pub fn from_env() -> Self {
        let config = Config::load();
        Self::from_config(&config, config.default_model().unwrap_or("echo"))
    }

    pub fn from_config(config: &Config, model: &str) -> Self {
        let provider = if model.contains('/') {
            model.split('/').next().unwrap_or("mock")
        } else {
            config.provider.as_deref().unwrap_or("mock")
        };
        Self::build_with_config(provider, model, config).0
    }

    pub fn switch(&mut self, provider: &str, model: &str) {
        *self = Self::build(provider, model).0;
    }

    pub fn name(&self) -> &'static str {
        match self {
            AnyProvider::Mock(_) => "mock",
            AnyProvider::OpenAi(_) => "openai",
        }
    }

    pub fn model(&self) -> String {
        match self {
            AnyProvider::Mock(_) => "echo".to_string(),
            AnyProvider::OpenAi(p) => p.model().to_string(),
        }
    }
}

impl Provider for AnyProvider {
    async fn generate<F>(&self, messages: Vec<Message>, on_chunk: F) -> Result<()>
    where
        F: FnMut(ResponseChunk) + Send,
    {
        match self {
            AnyProvider::Mock(p) => p.generate(messages, on_chunk).await,
            AnyProvider::OpenAi(p) => p.generate(messages, on_chunk).await,
        }
    }
}

/// Default timeout for API key validation requests.
pub const VALIDATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// Validate an API key by calling the provider's `/models` endpoint.
/// Returns a list of available model IDs on success.
///
/// Fails after [`VALIDATION_TIMEOUT`] so the UI never gets stuck waiting
/// for an unreachable or unresponsive provider.
pub async fn validate_api_key(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    validate_api_key_with_timeout(base_url, api_key, VALIDATION_TIMEOUT).await
}

/// Validate an API key with a configurable request timeout.
///
/// The timeout covers the full request lifecycle: DNS, connect, TLS,
/// request send, response headers, and response body. It is enforced via
/// `tokio::time::timeout` around the whole operation, and also via
/// `reqwest`'s client-level timeout and an explicit connect timeout, so
/// the UI can never get stuck waiting on a hanging or unreachable host.
pub async fn validate_api_key_with_timeout(
    base_url: &str,
    api_key: &str,
    timeout: std::time::Duration,
) -> Result<Vec<String>> {
    let fut = async move {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(timeout)
            .build()?;
        let url = format!("{}/models", base_url.trim_end_matches('/'));
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("API validation failed: {}", text);
        }

        let json: serde_json::Value = resp.json().await?;
        let models = json
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok::<Vec<String>, anyhow::Error>(models)
    };

    match tokio::time::timeout(timeout, fut).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!("API validation timed out after {}s", timeout.as_secs()),
    }
}

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod tests;
