use runie_ai::providers::{MockProvider, OpenAiProvider, AnthropicProvider, GenAiProvider, RigProvider, MiniMaxProvider};
use runie_ai::Provider;
use runie_core::RunieError;
use crate::settings::Settings;

pub(crate) fn validate_api_key(key: &str, provider: &str) -> Result<String, RunieError> {
    // Check for obvious garbage values
    if key.contains("cargo run") || key.contains("RUST_BACKTRACE") {
        return Err(RunieError::Provider(format!(
            "Invalid API key for {}: looks like a shell command was pasted into the config file. \
             Please edit your config and set a real API key.",
            provider
        )));
    }
    if key == "YOUR_MINIMAX_API_KEY_HERE" || key == "YOUR_API_KEY_HERE" || key.is_empty() {
        return Err(RunieError::Provider(format!(
            "API key not set for {}. Please set it in your config file or use the {}_API_KEY environment variable.",
            provider, provider.to_uppercase()
        )));
    }
    if key.len() < 10 {
        return Err(RunieError::Provider(format!(
            "API key for {} looks too short ({} chars). Please check your config.",
            provider, key.len()
        )));
    }
    Ok(key.to_string())
}

pub fn create_provider(mock: bool, settings: &Settings) -> Result<Box<dyn Provider>, RunieError> {
    if mock {
        return Ok(Box::new(MockProvider::new()));
    }

    match settings.provider.as_str() {
        "openai" => {
            let api_key = settings.api_key.clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| RunieError::Provider("OpenAI API key required. Set OPENAI_API_KEY env var or use --api-key".to_string()))?;
            let api_key = validate_api_key(&api_key, "openai")?;
            let provider = OpenAiProvider::new(api_key, settings.model.clone());
            Ok(Box::new(provider))
        }
        "anthropic" => {
            let api_key = settings.api_key.clone()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .ok_or_else(|| RunieError::Provider("Anthropic API key required. Set ANTHROPIC_API_KEY env var or use --api-key".to_string()))?;
            let api_key = validate_api_key(&api_key, "anthropic")?;
            let provider = AnthropicProvider::new(api_key, settings.model.clone());
            Ok(Box::new(provider))
        }
        "minimax" => {
            let api_key = settings.api_key.clone()
                .or_else(|| std::env::var("MINIMAX_API_KEY").ok())
                .ok_or_else(|| RunieError::Provider("MiniMax API key required. Set MINIMAX_API_KEY env var or use --api-key".to_string()))?;
            let api_key = validate_api_key(&api_key, "minimax")?;
            let provider = MiniMaxProvider::new(api_key, settings.model.clone());
            Ok(Box::new(provider))
        }
        "google" => {
            let provider = GenAiProvider::new(settings.model.clone());
            Ok(Box::new(provider))
        }
        other => {
            let api_key = settings.api_key.clone()
                .ok_or_else(|| RunieError::Provider(format!("API key required for provider: {}", other)))?;
            let provider = RigProvider::new(other, &api_key, &settings.model)
                .map_err(|e| RunieError::Provider(e.to_string()))?;
            Ok(Box::new(provider))
        }
    }
}