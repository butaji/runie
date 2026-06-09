#![warn(clippy::all)]

//! Runie Provider - Concrete LLM provider implementations

pub mod mock;
pub mod model;
pub mod openai;
pub mod config;

pub use mock::MockProvider;
pub use model::{ModelId, ModelRegistry};
pub use openai::OpenAiProvider;
pub use config::Config;

use anyhow::Result;
use runie_core::provider::{Message, Provider, ResponseChunk};

/// Runtime provider selection — closed enum for static dispatch.
pub enum AnyProvider {
    Mock(MockProvider),
    OpenAi(OpenAiProvider),
}

impl AnyProvider {
    fn build(provider: &str, model: &str) -> Self {
        let config = Config::load();
        Self::build_with_config(provider, model, &config)
    }

    fn build_with_config(provider: &str, model: &str, config: &Config) -> Self {
        if let Some(custom) = config.model_providers.get(provider) {
            return Self::OpenAi(OpenAiProvider::new(
                custom.api_key.clone(),
                model,
            ).with_base_url(custom.base_url.clone()));
        }
        match provider {
            "openai" => {
                let key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
                if key.is_empty() {
                    eprintln!("Warning: OPENAI_API_KEY not set, falling back to mock");
                    Self::Mock(MockProvider::default())
                } else {
                    Self::OpenAi(OpenAiProvider::new(key, model))
                }
            }
            _ => {
                if std::env::var("RUNIE_MOCK_DELAY").is_ok() {
                    Self::Mock(MockProvider::with_delay(500, 3000))
                } else {
                    Self::Mock(MockProvider::default())
                }
            }
        }
    }

    pub fn new(provider: &str, model: &str) -> Self {
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
        Self::build_with_config(provider, model, config)
    }

    pub fn switch(&mut self, provider: &str, model: &str) {
        *self = Self::build(provider, model);
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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod config_tests;
