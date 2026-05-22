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
            let provider = OpenAiProvider::new(api_key, settings.model.clone());
            Ok(Box::new(provider))
        }
        "anthropic" => {
            let api_key = settings.api_key.clone()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .ok_or("Anthropic API key required. Set ANTHROPIC_API_KEY env var or use --api-key")?;
            let provider = AnthropicProvider::new(api_key, settings.model.clone());
            Ok(Box::new(provider))
        }
        other => Err(format!("Unknown provider: {}. Use 'openai' or 'anthropic'", other)),
    }
}