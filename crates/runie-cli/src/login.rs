//! Login command — Configure a provider with API key.
//!
//! Guides users through provider setup:
//! 1. Lists available providers
//! 2. Accepts provider name and API key
//! 3. Stores API key in OS keyring
//! 4. Updates config.toml with provider settings

use anyhow::{Context, Result};
use std::io::{self, Write};

/// Run the login command interactively.
///
/// The API key is always read from the interactive prompt (or the provider's
/// env var by the user beforehand) — never from a command-line argument, since
/// argv is visible via `ps` and shell history.
pub async fn run(provider_name: Option<String>) -> Result<()> {
    let providers = runie_core::provider::known_providers();

    // If no provider specified, show interactive picker
    let provider = match provider_name {
        Some(name) => {
            let normalized = name.to_lowercase();
            runie_core::provider::find_provider(&normalized).ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown provider '{}'. Available: {}",
                    name,
                    providers
                        .iter()
                        .map(|p| p.key.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?
        }
        None => {
            print_providers_list(&providers);
            let selection = prompt_provider_selection(&providers)?;
            providers.into_iter().find(|p| p.key == selection).unwrap()
        }
    };

    // Read the API key interactively (never from argv — avoids leaking it via
    // `ps` or shell history). For non-interactive setups, the user can export
    // the provider's env var instead.
    println!("\nConfiguring {} ({})", provider.display_name, provider.key);
    println!("Environment variable: {}", provider.env_var);
    let key = prompt_api_key(&provider)?;

    // Store in keyring
    println!("\nStoring API key in OS keyring...");
    runie_core::auth::set_keyring(&provider.key, &key).context("Failed to store API key in keyring")?;

    // Update config
    println!("Updating config...");
    update_config(&provider.key)?;

    println!("\n✅ {} is now configured!", provider.display_name);
    println!("   Run `runie inspect` to verify your setup.");
    println!(
        "   Or set the {} environment variable for quick testing.",
        provider.env_var
    );

    Ok(())
}

fn print_providers_list(providers: &[runie_core::provider::ProviderMeta]) {
    println!("\nAvailable providers:");
    println!("{}", "─".repeat(50));
    for (i, provider) in providers.iter().enumerate() {
        println!(
            "  {:2}. {:15} ({})",
            i + 1,
            provider.key,
            provider.display_name
        );
    }
    println!("{}", "─".repeat(50));
}

fn prompt_provider_selection(providers: &[runie_core::provider::ProviderMeta]) -> Result<String> {
    print!("\nSelect provider (number): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let selection = input.trim();

    let index: usize = selection.parse().context("Please enter a number")?;

    if index == 0 || index > providers.len() {
        anyhow::bail!("Invalid selection. Choose 1-{}", providers.len());
    }

    Ok(providers[index - 1].key.clone())
}

fn prompt_api_key(provider: &runie_core::provider::ProviderMeta) -> Result<String> {
    println!("\nEnter your {} API key:", provider.display_name);
    print!("  API key: ");
    io::stdout().flush()?;

    // Use simple read without echo control for simplicity
    let mut key = String::new();
    io::stdin().read_line(&mut key)?;
    let key = key.trim().to_string();

    if key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    Ok(key)
}

fn update_config(provider_key: &str) -> Result<()> {
    let mut config = runie_core::config::Config::load(None);

    // Set provider as default
    config.provider = Some(provider_key.to_string());

    // Add provider to model_providers if not already present
    let provider_meta = runie_core::provider::find_provider(provider_key)
        .ok_or_else(|| anyhow::anyhow!("Provider {} not found in registry", provider_key))?;

    let mut model_providers = std::mem::take(&mut config.model_providers);

    // Ensure we have an entry for this provider
    model_providers
        .entry(provider_key.to_string())
        .or_insert_with(|| runie_core::config::ModelProvider {
            provider_type: None,
            base_url: provider_meta.base_url,
            models: provider_meta
                .models
                .iter()
                .map(|m| m.name.clone())
                .collect(),
            headers: std::collections::HashMap::new(),
            context_window_fallbacks: Vec::new(),
        });

    config.model_providers = model_providers;

    // Set default model if not set
    if config.models.default.is_none() {
        if let Some(first_model) = provider_meta.models.first() {
            config.models.default = Some(first_model.name.clone());
        }
    }

    config.save()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use runie_core::provider::{find_provider, known_providers};

    #[test]
    fn lists_available_providers() {
        let providers = known_providers();
        assert!(!providers.is_empty(), "Should have at least one provider");
        assert!(providers.iter().any(|p| p.key == "openai"));
    }

    #[test]
    fn finds_known_provider() {
        let p = find_provider("openai");
        assert!(p.is_some());
        let p = p.unwrap();
        assert_eq!(p.display_name, "OpenAI");
        assert!(!p.base_url.is_empty());
    }

    #[test]
    fn finds_unknown_provider_returns_none() {
        let p = find_provider("nonexistent-provider");
        assert!(p.is_none());
    }
}
