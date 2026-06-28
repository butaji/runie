//! Fuzzy filtering score for panel items.

use super::item::PanelItem;
use runie_core::fuzzy;

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
    fuzzy::score(query, label).map(|s| s as isize)
}

/// Score any panel item, using command-aware matching for palette entries.
pub fn item_match_score(item: &PanelItem, query: &str) -> Option<isize> {
    match item {
        PanelItem::Command {
            name, desc, label, ..
        } => command_match_score(name, desc, label, query),
        _ => match_score(item.label()?, query),
    }
}

/// Score a command-palette entry, allowing the query to include arguments
/// after the command name (e.g. "model gpt-4o-mini" should still match the
/// `/model` command).
fn command_match_score(name: &str, desc: &str, label: &str, query: &str) -> Option<isize> {
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
    fuzzy::score(query, name).map(|s| s as isize)
}
