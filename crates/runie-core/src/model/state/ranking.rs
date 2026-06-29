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
            let base = fuzzy_score(query, &cmd.name)
                .or_else(|| fuzzy_score(query, &cmd.desc))?;
            let usage = command_usage.get(&cmd.name);
            let score = compute_ranking_score(query, cmd, usage) + base * 100;
            Some((cmd.name.clone(), score))
        })
        .collect();
    ranked.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    ranked.into_iter().take(limit).collect()
}

pub fn has_provider_credentials(config: &crate::config::Config, provider: &str) -> bool {
    config
        .model_providers
        .get(provider)
        .map(|p| !p.api_key.is_empty())
        .unwrap_or(false)
}
