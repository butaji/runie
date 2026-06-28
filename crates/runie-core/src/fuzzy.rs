//! Fuzzy string matching using `sublime_fuzzy`.
//!
//! Provides subsequence scoring for @-ref completions and panel filtering.
//! File-path fuzzy matching stays with FFF.

/// Score a fuzzy match between `query` and `candidate`.
pub fn score(query: &str, candidate: &str) -> Option<i32> {
    sublime_fuzzy::best_match(query, candidate)
        .map(|m| m.score() as i32)
}

/// Backward-compatible alias.
#[deprecated(since = "0.2.16", note = "Use `score()` instead")]
pub type NucleoMatcher = FuzzyMatcher;

/// Fuzzy matcher for non-file items.
pub struct FuzzyMatcher {
    _private: (),
}

impl FuzzyMatcher {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Score and rank candidates. Returns matched items sorted by score.
    pub fn filter<'a>(&self, query: &str, candidates: &'a [&str], limit: usize) -> Vec<&'a str> {
        if query.is_empty() {
            return candidates.iter().take(limit).copied().collect();
        }
        let mut scored: Vec<(i32, usize, &'a str)> = candidates
            .iter()
            .enumerate()
            .filter_map(|(idx, c)| score(query, c).map(|s| (s, idx, *c)))
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.0));
        scored.into_iter().take(limit).map(|(_, _, c)| c).collect()
    }
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Backward-compatible alias for `score`.
#[deprecated(since = "0.2.16", note = "Use `score()` instead")]
pub fn fuzzy_match(query: &str, candidate: &str) -> Option<i32> {
    score(query, candidate)
}

/// Filter and rank candidates by fuzzy match score.
pub fn fuzzy_filter<'a>(query: &str, candidates: &[&'a str], limit: usize) -> Vec<&'a str> {
    let mut scored: Vec<(i32, &'a str)> = candidates
        .iter()
        .filter_map(|c| score(query, c).map(|s| (s, *c)))
        .collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    scored.into_iter().take(limit).map(|(_, c)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        let result = fuzzy_match("hello", "hello");
        assert!(result.is_some(), "exact match should succeed");
        assert!(*result.as_ref().unwrap() > 0, "exact match score should be positive");
    }

    #[test]
    fn no_match() {
        assert_eq!(fuzzy_match("xyz", "abc"), None);
    }

    #[test]
    fn filter_ranking() {
        let candidates = vec!["main.rs", "lib.rs", "README.md"];
        let results = fuzzy_filter("mr", &candidates, 10);
        assert!(!results.is_empty());
        assert_eq!(results[0], "main.rs");
    }

    #[test]
    fn fuzzy_matcher_scores_panel_items() {
        let matcher = FuzzyMatcher::new();
        let items = &["Settings", "Session List", "Model Selector", "About"];
        let results = matcher.filter("set", items, 10);
        assert!(!results.is_empty());
        // "Settings" should rank highest for "set" due to prefix/consecutive match
        assert_eq!(results[0], "Settings");
    }

    #[test]
    fn fuzzy_matcher_handles_unicode() {
        let matcher = FuzzyMatcher::new();
        let items = &["日本語", "English", "Español"];
        let results = matcher.filter("en", items, 10);
        assert!(!results.is_empty());
        assert_eq!(results[0], "English");
    }
}
