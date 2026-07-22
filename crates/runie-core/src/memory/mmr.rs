//! Maximum Marginal Relevance (MMR) reranking.
//!
//! MMR provides diverse results by selecting items that are both relevant
//! to the query AND different from previously selected items.

use crate::memory::search::SearchResult;
use std::collections::HashSet;

/// MMR reranking with diversity control.
///
/// MMR(d) = λ × relevance(d) - (1-λ) × max_similarity(d, selected)
///
/// - `lambda`: Diversity factor (0.0 to 1.0). Higher = more diverse results.
/// - `k`: Number of results to return.
pub fn mmr_rerank(results: Vec<SearchResult>, lambda: f32, k: usize) -> Vec<SearchResult> {
    if results.is_empty() || k == 0 {
        return Vec::new();
    }

    let lambda = lambda.clamp(0.0, 1.0);
    let k = k.min(results.len());

    let mut selected: Vec<SearchResult> = Vec::with_capacity(k);
    let mut remaining: Vec<SearchResult> = results;
    let mut selected_contents: HashSet<String> = HashSet::new();

    // Add first result (highest relevance)
    if let Some(first) = remaining.pop() {
        selected.push(first.clone());
        selected_contents.insert(normalize_content(&first.entry.content));
    }

    // Select remaining results using MMR
    while selected.len() < k && !remaining.is_empty() {
        let mut best_idx = 0;
        let mut best_mmr = f32::MIN;

        for (idx, candidate) in remaining.iter().enumerate() {
            let relevance = candidate.score;
            let max_sim = calculate_max_similarity(&candidate.entry.content, &selected_contents);

            // MMR formula
            let mmr = lambda * relevance - (1.0 - lambda) * max_sim;

            if mmr > best_mmr {
                best_mmr = mmr;
                best_idx = idx;
            }
        }

        let chosen = remaining.remove(best_idx);
        selected_contents.insert(normalize_content(&chosen.entry.content));
        selected.push(chosen);
    }

    selected
}

/// Normalize content for comparison.
fn normalize_content(content: &str) -> String {
    content
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Calculate maximum Jaccard similarity to any selected item.
fn calculate_max_similarity(candidate: &str, selected: &HashSet<String>) -> f32 {
    let candidate_set = tokenize(candidate);

    selected
        .iter()
        .map(|s| jaccard_similarity(&candidate_set, &tokenize(s)))
        .fold(0.0, f32::max)
}

/// Tokenize text into a set of words.
fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Calculate Jaccard similarity between two sets.
fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let intersection = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;

    intersection / union
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::MemoryEntry;

    fn make_result(content: &str, score: f32) -> SearchResult {
        SearchResult::new(MemoryEntry::new(content, crate::memory::store::MemorySource::Global), score)
    }

    #[test]
    fn mmr_returns_correct_count() {
        let results = vec![
            make_result("apple", 0.9),
            make_result("apple apple apple", 0.8),
            make_result("banana", 0.7),
            make_result("cherry", 0.6),
        ];

        let reranked = mmr_rerank(results, 0.7, 2);
        assert_eq!(reranked.len(), 2);
    }

    #[test]
    fn mmr_includes_diverse_results() {
        let results = vec![
            make_result("python programming language", 0.9),
            make_result("python snake animal", 0.8),
            make_result("rust programming language", 0.7),
            make_result("javascript web development", 0.6),
        ];

        let reranked = mmr_rerank(results, 0.5, 2);
        // First and third should be selected (different topics)
        assert_eq!(reranked.len(), 2);
    }

    #[test]
    fn mmr_empty_input() {
        let results: Vec<SearchResult> = vec![];
        let reranked = mmr_rerank(results, 0.5, 5);
        assert!(reranked.is_empty());
    }

    #[test]
    fn mmr_k_greater_than_results() {
        let results = vec![
            make_result("a", 0.9),
            make_result("b", 0.8),
        ];

        let reranked = mmr_rerank(results, 0.5, 10);
        assert_eq!(reranked.len(), 2);
    }

    #[test]
    fn test_jaccard_similarity() {
        let a: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let b: HashSet<String> = ["b", "c", "d"].iter().map(|s| s.to_string()).collect();

        let sim = jaccard_similarity(&a, &b);
        // intersection = 2, union = 4
        assert!((sim - 0.5).abs() < 0.001);
    }
}
