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
    // The mock provider does not require credentials.
    if provider == "mock" && crate::provider::is_mock_enabled() {
        return true;
    }
    // Use the same resolution chain as CredentialResolver so auth.json and all
    // other sources are consistently detected. This fixes the bug where a provider
    // with valid credentials in auth.json was not detected during startup.
    let resolver = crate::auth::CredentialResolver::new();
    let has = resolver.resolve_api_key(provider).is_some();
    tracing::debug!(
        provider,
        has_credentials = has,
        "has_provider_credentials"
    );
    has
}

#[cfg(test)]
mod ranking_tests {
    use super::*;

    #[test]
    fn test_has_provider_credentials_auth_json_priority() {
        // has_provider_credentials uses CredentialResolver which checks auth.json.
        // Use RUNIE_AUTH_FILE to isolate from real auth.json on this machine.
        let dir = tempfile::tempdir().unwrap();
        let fake_auth = dir.path().join("auth.json");
        std::fs::write(&fake_auth, r#"{"other": {"token": "sk-other"}}"#).unwrap();
        std::env::set_var("RUNIE_AUTH_FILE", &fake_auth);

        let config = crate::config::Config::default();
        // "minimax" is not in the fake auth file, so should return false.
        let has = has_provider_credentials(&config, "minimax");
        assert!(!has, "minimax should not have credentials in isolated test auth file");

        std::env::remove_var("RUNIE_AUTH_FILE");
    }

    #[test]
    fn test_has_provider_credentials_mock_requires_no_key() {
        let config = crate::config::Config::default();
        crate::provider::set_mock_enabled(true);
        assert!(
            has_provider_credentials(&config, "mock"),
            "mock provider should not require credentials when mock is enabled"
        );
        crate::provider::set_mock_enabled(false);
        assert!(
            !has_provider_credentials(&config, "mock"),
            "mock provider should require credentials when mock is disabled"
        );
    }
}
