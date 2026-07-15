//! Fuzzy filtering score for panel items.

use crate::dialog::PanelItem;

/// Score how well a `label` matches a `query`. Higher is better.
/// Priority: startsWith > contains > fuzzy character-order match.
pub fn match_score(label: &str, query: &str) -> Option<isize> {
    if query.is_empty() {
        return Some(0);
    }
    let label_lower = label.to_lowercase();
    let query_lower = query.to_lowercase();

    if label_lower.starts_with(&query_lower) {
        return Some(10_000 + (100 - label.len() as isize).max(0));
    }
    if label_lower.contains(&query_lower) {
        return Some(5_000 + (100 - label.len() as isize).max(0));
    }
    fuzzy_score(query, label).map(|s| s as isize)
}

/// Score any panel item, using command-aware matching for palette entries.
pub fn item_match_score(item: &PanelItem, query: &str) -> Option<isize> {
    match item {
        PanelItem::Command {
            name,
            desc,
            label,
            aliases,
            ..
        } => command_match_score(name, desc, label, aliases, query),
        _ => match_score(item.label()?, query),
    }
}

/// Score a command-palette entry, allowing the query to include arguments
/// after the command name (e.g. "model gpt-4o-mini" should still match the
/// `/model` command).
fn command_match_score(
    name: &str,
    desc: &str,
    label: &str,
    aliases: &[String],
    query: &str,
) -> Option<isize> {
    if query.is_empty() {
        return Some(0);
    }
    let name_lower = name.to_lowercase();
    let query_lower = query.to_lowercase();

    // Exact "name args" input: highest priority.
    if query_lower.starts_with(&name_lower)
        && query_lower
            .get(name_lower.len()..)
            .is_some_and(|r| r.starts_with(' '))
    {
        return Some(20_000 + (100 - name.len() as isize).max(0));
    }

    // Exact alias match (e.g. "exit" -> /quit): treat as a prefix match on the
    // canonical name so aliases are first-class palette filters.
    let query_trimmed = query_lower.trim_start_matches('/');
    for alias in aliases {
        if alias.eq_ignore_ascii_case(query_trimmed) {
            return Some(10_000 + (100 - name.len() as isize).max(0));
        }
    }

    // Prefix match on the command name ("mod" matches "model").
    if name_lower.starts_with(&query_lower) {
        return Some(10_000 + (100 - name.len() as isize).max(0));
    }

    // Match on full label/description.
    if let Some(score) = match_score(label, query) {
        return Some(score);
    }
    if let Some(score) = match_score(desc, query) {
        return Some(score.max(1_000));
    }

    // Fuzzy match against the command name so "mdl" can still find "model".
    fuzzy_score(query, name).map(|s| s as isize)
}

/// Score a fuzzy match between `query` and `candidate` using `sublime_fuzzy`.
fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    sublime_fuzzy::best_match(query, candidate).map(|m| m.score() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::{ItemAction, PanelItem};

    fn command_item(name: &str, desc: &str, aliases: &[&str]) -> PanelItem {
        let label = format!("{} {}", name, desc);
        PanelItem::Command {
            name: name.into(),
            desc: desc.into(),
            label,
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
            action: ItemAction::Close,
        }
    }

    #[test]
    fn alias_exact_match_scores_high() {
        let quit = command_item("quit", "Quit application", &["q", "exit"]);
        let quit_score = item_match_score(&quit, "exit").expect("alias should match");
        let export = command_item("export", "Export session to JSON", &[]);
        let export_score = item_match_score(&export, "exit");
        assert!(
            export_score.is_none() || quit_score > export_score.unwrap(),
            "quit via alias 'exit' must outrank unrelated export command"
        );
    }

    #[test]
    fn alias_match_with_leading_slash() {
        let quit = command_item("quit", "Quit application", &["q", "exit"]);
        let score = item_match_score(&quit, "/exit").expect("'/exit' should match quit alias");
        assert!(score >= 10_000, "exact alias should score like a prefix match");
    }

    #[test]
    fn canonical_name_still_matches() {
        let quit = command_item("quit", "Quit application", &["q", "exit"]);
        let score = item_match_score(&quit, "quit").expect("canonical name should match");
        assert!(score >= 10_000, "canonical prefix match should score highly");
    }
}
