//! Fuzzy string matching for @-ref completions and panel filtering.
//!
//! Uses nucleo-matcher for non-file items (command palette, model selector,
//! dialog panels). File-path fuzzy matching stays with FFF.

use nucleo_matcher::{Matcher, pattern::{Atom, CaseMatching, Normalization}};

/// Nucleo matcher for non-file items. Reuse across calls to avoid allocations.
pub struct NucleoMatcher {
    matcher: Matcher,
}

impl NucleoMatcher {
    pub fn new() -> Self { Self { matcher: Matcher::new(nucleo_matcher::Config::DEFAULT) } }
    
    /// Score and rank candidates. Returns matched items sorted by score.
    pub fn filter<'a>(&self, query: &str, candidates: &'a [&str], limit: usize) -> Vec<&'a str> {
        if query.is_empty() { return candidates.iter().take(limit).copied().collect(); }
        let atom = Atom::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut chars = Vec::new();
        let mut scored: Vec<(u16, usize, &'a str)> = candidates
            .iter()
            .enumerate()
            .filter_map(|(idx, c)| {
                chars.clear();
                chars.extend(c.chars());
                let haystack = nucleo_matcher::Utf32Str::new(c, &mut chars);
                atom.score(haystack, &mut self.matcher.clone())
                    .map(|score| (score, idx, *c))
            })
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.0));
        scored.into_iter().take(limit).map(|(_, _, c)| c).collect()
    }
}

impl Default for NucleoMatcher {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Legacy scorer (kept for file-path matching via FFF)
// ---------------------------------------------------------------------------

/// Score a fuzzy match between `query` and `candidate`.
pub fn score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() { return Some(0); }
    let query_lower = query.to_lowercase();
    let cand_lower = candidate.to_lowercase();
    let mut score = 0i32;
    let mut ci = 0usize;
    for (qi, qc) in query_lower.chars().enumerate() {
        let (new_ci, step) = score_query_char(qc, qi, ci, &cand_lower)?;
        ci = new_ci;
        score += step;
    }
    score -= (cand_lower.len() - query_lower.len()) as i32;
    Some(score)
}

fn score_query_char(qc: char, qi: usize, ci: usize, cand_lower: &str) -> Option<(usize, i32)> {
    let pos = cand_lower[ci..].find(qc)?;
    let abs_pos = ci + pos;
    let mut step = 10i32;
    if abs_pos == 0 { step += 5; }
    step += word_boundary_bonus(cand_lower, abs_pos);
    if qi > 0 { step -= pos as i32; }
    Some((abs_pos + 1, step))
}

fn word_boundary_bonus(cand_lower: &str, abs_pos: usize) -> i32 {
    if abs_pos == 0 { return 0; }
    let prev = cand_lower.as_bytes()[abs_pos - 1];
    if matches!(prev, b'.' | b'/' | b'-' | b'_') { 10 } else { 0 }
}

/// Backward-compatible alias for `score`.
pub fn fuzzy_match(query: &str, candidate: &str) -> Option<i32> { score(query, candidate) }

/// Filter and rank candidates by fuzzy match score.
pub fn fuzzy_filter<'a>(query: &str, candidates: &[&'a str], limit: usize) -> Vec<&'a str> {
    let mut scored: Vec<(i32, &'a str)> = candidates
        .iter()
        .filter_map(|c| fuzzy_match(query, c).map(|s| (s, *c)))
        .collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    scored.into_iter().take(limit).map(|(_, c)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() { assert_eq!(fuzzy_match("hello", "hello"), Some(55)); }
    #[test]
    fn no_match() { assert_eq!(fuzzy_match("xyz", "abc"), None); }
    #[test]
    fn filter_ranking() {
        let candidates = vec!["main.rs", "lib.rs", "README.md"];
        let results = fuzzy_filter("mr", &candidates, 10);
        assert_eq!(results[0], "main.rs");
    }

    #[test]
    fn nucleo_scores_panel_items() {
        let matcher = NucleoMatcher::new();
        let items = &["Settings", "Session List", "Model Selector", "About"];
        let results = matcher.filter("set", items, 10);
        assert!(!results.is_empty());
        assert_eq!(results[0], "Settings");
    }

    #[test]
    fn nucleo_handles_unicode_query() {
        let matcher = NucleoMatcher::new();
        let items = &["日本語", "English", "Español"];
        let results = matcher.filter("日本", items, 10);
        assert!(!results.is_empty());
        assert_eq!(results[0], "日本語");
    }
}
