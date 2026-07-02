#![allow(clippy::all)]
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

pub fn has_provider_credentials(_config: &crate::config::Config, provider: &str) -> bool {
    // Check env var first (takes priority in the credential resolution chain)
    let env_key = format!("{}_API_KEY", provider.to_uppercase());
    if let Ok(val) = std::env::var(&env_key) {
        if !val.is_empty() {
            return true;
        }
    }
    // Then keyring
    // Check keyring first
    if crate::auth::AuthStorage::get_keyring_token(provider).is_some() {
        return true;
    }
    // API keys are no longer stored in config - only keyring/env
    false
}

#[cfg(test)]
mod ranking_tests {
    use super::*;

    #[test]
    fn test_has_provider_credentials_checks_keyring() {
        // has_provider_credentials checks keyring first
        // Config no longer stores api_key - only keyring/env
        let config = crate::config::Config::default();
        // Without keyring, should return false
        let has = has_provider_credentials(&config, "nonexistent");
        assert!(!has, "should not find credentials without keyring");
    }
}
