use super::helpers::compute_ranking_score;
use super::CommandUsage;
use std::collections::HashMap;

fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    sublime_fuzzy::best_match(query, candidate).map(|m| m.score() as i32)
}

pub fn rank_commands_empty_query(
    command_usage: &HashMap<String, CommandUsage>,
    all: &[&crate::commands::CommandDef],
    limit: usize,
) -> Vec<(String, i32)> {
    let mut ranked: Vec<_> = all
        .iter()
        .map(|cmd| {
            let usage = command_usage.get(&cmd.name);
            let score = compute_ranking_score("", cmd, usage);
            (cmd.name.clone(), score)
        })
        .collect();
    ranked.sort_by_key(|(name, score)| (std::cmp::Reverse(*score), name.clone()));
    ranked.into_iter().take(limit).collect()
}

pub fn rank_commands_with_query(
    command_usage: &HashMap<String, CommandUsage>,
    query: &str,
    all: &[&crate::commands::CommandDef],
    limit: usize,
) -> Vec<(String, i32)> {
    let mut ranked: Vec<_> = all
        .iter()
        .filter_map(|cmd| {
            let base = fuzzy_score(query, &cmd.name).or_else(|| fuzzy_score(query, &cmd.desc))?;
            let usage = command_usage.get(&cmd.name);
            let score = compute_ranking_score(query, cmd, usage) + base * 100;
            Some((cmd.name.clone(), score))
        })
        .collect();
    ranked.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    ranked.into_iter().take(limit).collect()
}

pub fn has_provider_credentials(config: &crate::config::Config, provider: &str) -> bool {
    // Check env var first (takes priority in the credential resolution chain)
    let env_key = format!("{}_API_KEY", provider.to_uppercase());
    if let Ok(val) = std::env::var(&env_key) {
        if !val.is_empty() {
            return true;
        }
    }
    // Then keyring
    if crate::auth::AuthStorage::get_keyring_token(provider).is_some() {
        return true;
    }
    // Finally config file (legacy fallback)
    config
        .model_providers
        .get(provider)
        .map(|p| !p.api_key.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod ranking_tests {
    use super::*;

    #[test]
    fn test_has_provider_credentials_with_config_api_key() {
        // Create a config with api_key in model_providers
        let mut config = crate::config::Config::default();
        config.provider = Some("openai".to_string());
        config.model_providers.insert("openai".into(), crate::config::ModelProvider {
            provider_type: None,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            models: vec!["gpt-4o".to_string()],
        });

        let has = has_provider_credentials(&config, "openai");
        assert!(has, "should find credentials in config model_providers");
    }
}
