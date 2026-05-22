use runie_ai::providers::{MockProvider, OpenAiProvider, AnthropicProvider};
use runie_ai::Provider;
use crate::settings::Settings;

pub fn create_provider(mock: bool, settings: &Settings) -> Result<Box<dyn Provider>, String> {
    if mock {
        return Ok(Box::new(MockProvider::new()));
    }

    match settings.provider.as_str() {
        "openai" => {
            let api_key = settings.api_key.clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or("OpenAI API key required. Set OPENAI_API_KEY env var or use --api-key")?;
            let mut provider = OpenAiProvider::new(api_key, settings.model.clone());
            if let Some(ref base_url) = settings.base_url {
                provider = provider.with_base_url(base_url.clone());
            }
            Ok(Box::new(provider))
        }
        "anthropic" => {
            let api_key = settings.api_key.clone()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .ok_or("Anthropic API key required. Set ANTHROPIC_API_KEY env var or use --api-key")?;
            let mut provider = AnthropicProvider::new(api_key, settings.model.clone());
            if let Some(ref base_url) = settings.base_url {
                provider = provider.with_base_url(base_url.clone());
            }
            Ok(Box::new(provider))
        }
        other => Err(format!("Unknown provider: {}. Use 'openai' or 'anthropic'", other)),
    }
}