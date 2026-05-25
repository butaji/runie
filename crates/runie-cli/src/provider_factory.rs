use runie_ai::providers::{MockProvider, OpenAiProvider, AnthropicProvider, GenAiProvider, RigProvider};
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
        "google" => {
            let provider = GenAiProvider::new(settings.model.clone());
            Ok(Box::new(provider))
        }
        other => {
            let api_key = settings.api_key.clone()
                .ok_or(format!("API key required for provider: {}", other))?;
            let provider = RigProvider::new(other, &api_key, &settings.model)
                .map_err(|e| e.to_string())?;
            Ok(Box::new(provider))
        }
    }
}