use super::AppState;
use super::helpers::compute_ranking_score;

pub fn rank_commands_empty_query<'a>(
    state: &'a AppState,
    all: &[&'a crate::commands::CommandDef],
    limit: usize,
) -> Vec<(&'a crate::commands::CommandDef, i32)> {
    let mut ranked: Vec<_> = all
        .iter()
        .map(|cmd| {
            let usage = state.config.command_usage.get(&cmd.name);
            let score = compute_ranking_score("", cmd, usage);
            (*cmd, score)
        })
        .collect();
    ranked.sort_by_key(|(cmd, score)| (std::cmp::Reverse(*score), &cmd.category, &cmd.name));
    ranked.into_iter().take(limit).collect()
}

pub fn rank_commands_with_query<'a>(
    state: &'a AppState,
    query: &str,
    all: &[&'a crate::commands::CommandDef],
    limit: usize,
) -> Vec<(&'a crate::commands::CommandDef, i32)> {
    let mut ranked: Vec<_> = all
        .iter()
        .filter_map(|cmd| {
            let base = crate::fuzzy::fuzzy_match(query, &cmd.name)
                .or_else(|| crate::fuzzy::fuzzy_match(query, &cmd.desc))?;
            let usage = state.config.command_usage.get(&cmd.name);
            let score = compute_ranking_score(query, cmd, usage) + base * 100;
            Some((*cmd, score))
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
