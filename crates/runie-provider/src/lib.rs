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
    pub fn from_env() -> Self {
        let config = Config::load();
        let provider = config.provider.as_deref().unwrap_or("mock");
        let model = config.model.as_deref().unwrap_or("echo");

        match provider {
            "openai" => {
                let key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
                if key.is_empty() {
                    eprintln!("Warning: OPENAI_API_KEY not set, falling back to mock");
                    Self::Mock(MockProvider)
                } else {
                    Self::OpenAi(OpenAiProvider::new(key, model))
                }
            }
            _ => Self::Mock(MockProvider),
        }
    }

    pub fn switch(&mut self, provider: &str, model: &str) {
        *self = match provider {
            "openai" => {
                let key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
                if key.is_empty() {
                    eprintln!("Warning: OPENAI_API_KEY not set, falling back to mock");
                    Self::Mock(MockProvider)
                } else {
                    Self::OpenAi(OpenAiProvider::new(key, model))
                }
            }
            _ => Self::Mock(MockProvider),
        };
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
            AnyProvider::OpenAi(_) => "gpt-4o".to_string(), // TODO: store model in provider
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
