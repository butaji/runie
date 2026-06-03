use runie_ai::providers::{MockProvider, GenAiProvider, RigProvider, MiniMaxProvider, ReplyProvider};
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
        return Ok(Box::new(MockProvider::new().with_delay(500)));
    }

    match settings.provider.as_str() {
        "minimax" => create_minimax_provider(settings),
        "google" => create_google_provider(settings),
        "reply" => Ok(Box::new(ReplyProvider::with_default_fixtures()
            .map_err(|e| RunieError::Provider(e.to_string()))?)),
        other => create_rig_provider(other, settings),
    }
}

fn create_minimax_provider(settings: &Settings) -> Result<Box<dyn Provider>, RunieError> {
    let api_key = get_api_key(settings, "minimax", "MINIMAX_API_KEY")?;
    let provider = MiniMaxProvider::new(api_key, settings.model.clone());
    Ok(Box::new(provider))
}

fn create_google_provider(settings: &Settings) -> Result<Box<dyn Provider>, RunieError> {
    let provider = GenAiProvider::new(settings.model.clone());
    Ok(Box::new(provider))
}

fn create_rig_provider(other: &str, settings: &Settings) -> Result<Box<dyn Provider>, RunieError> {
    let api_key = settings.api_key.clone()
        .ok_or_else(|| RunieError::Provider(format!("API key required for provider: {}", other)))?;
    let provider = RigProvider::new(other, &api_key, &settings.model)
        .map_err(|e| RunieError::Provider(e.to_string()))?;
    Ok(Box::new(provider))
}

fn get_api_key(settings: &Settings, provider: &str, env_var: &str) -> Result<String, RunieError> {
    let api_key = settings.api_key.clone()
        .or_else(|| std::env::var(env_var).ok())
        .ok_or_else(|| RunieError::Provider(format!(
            "{} API key required. Set {} env var or use --api-key",
            provider.to_uppercase(),
            env_var
        )))?;
    validate_api_key(&api_key, provider)
}