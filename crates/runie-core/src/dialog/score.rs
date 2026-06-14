//! Fuzzy filtering score for panel items.

use crate::fuzzy;

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
