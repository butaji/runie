//! Memory search with query building and result ranking.

use super::store::{MemoryEntry, MemorySource};
use serde::{Deserialize, Serialize};

/// Search query parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search text.
    pub text: String,
    /// Filter by source.
    pub source: Option<MemorySource>,
    /// Filter by tags.
    pub tags: Vec<String>,
    /// Filter by workspace.
    pub workspace: Option<String>,
    /// Maximum results.
    pub limit: usize,
    /// Include recency boost.
    pub recency_boost: bool,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            source: None,
            tags: Vec::new(),
            workspace: None,
            limit: 10,
            recency_boost: true,
        }
    }
}

impl SearchQuery {
    /// Create a new search query.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    /// Filter by source.
    pub fn source(mut self, source: MemorySource) -> Self {
        self.source = Some(source);
        self
    }

    /// Filter by tags.
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Filter by workspace.
    pub fn workspace(mut self, workspace: impl Into<String>) -> Self {
        self.workspace = Some(workspace.into());
        self
    }

    /// Set result limit.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Search result with score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matching memory entry.
    pub entry: MemoryEntry,
    /// Relevance score (0.0 to 1.0).
    pub score: f32,
    /// Match highlights.
    pub highlights: Vec<String>,
}

impl SearchResult {
    /// Create a new search result.
    pub fn new(entry: MemoryEntry, score: f32) -> Self {
        Self {
            entry,
            score,
            highlights: Vec::new(),
        }
    }

    /// Add a highlight.
    pub fn with_highlight(mut self, highlight: impl Into<String>) -> Self {
        self.highlights.push(highlight.into());
        self
    }
}

/// Rank search results by relevance.
pub fn rank_results(entries: Vec<MemoryEntry>, query: &str) -> Vec<SearchResult> {
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = query_lower.split_whitespace().collect();

    let mut results: Vec<SearchResult> = entries
        .into_iter()
        .map(|entry| {
            let score = calculate_relevance(&entry, &query_lower, &terms);
            SearchResult::new(entry, score)
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    results
}

/// Calculate relevance score for an entry.
fn calculate_relevance(entry: &MemoryEntry, query: &str, terms: &[&str]) -> f32 {
    let content_lower = entry.content.to_lowercase();
    let mut score = 0.0;

    // Exact match bonus
    if content_lower.contains(query) {
        score += 0.4;
    }

    // Term frequency scoring
    for term in terms {
        let count = content_lower.matches(term).count() as f32;
        score += (count * 0.1).min(0.3);
    }

    // Importance boost
    score += entry.importance * 0.15;

    // Recency boost (decay over time)
    let hours_old = (chrono::Utc::now() - entry.accessed_at).num_hours() as f32;
    let recency = (-hours_old / (24.0 * 7.0)).exp(); // Decay over a week
    score += recency * 0.1;

    // Access frequency boost
    let access = (entry.access_count as f32).ln() * 0.05;
    score += access;

    score.clamp(0.0, 1.0)
}

/// Extract highlights from matching content.
pub fn extract_highlights(content: &str, query: &str, context_chars: usize) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let content_lower = content.to_lowercase();
    let mut highlights = Vec::new();

    let mut start = 0;
    while let Some(pos) = content_lower[start..].find(&query_lower) {
        let abs_pos = start + pos;
        let ctx_start = abs_pos.saturating_sub(context_chars);
        let ctx_end = (abs_pos + query.len() + context_chars).min(content.len());

        let highlight = &content[ctx_start..ctx_end];
        highlights.push(format!("...{}...", highlight));

        start = abs_pos + query.len();
    }

    highlights.truncate(3); // Limit highlights
    highlights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_query_builder() {
        let query = SearchQuery::new("test query")
            .source(MemorySource::Global)
            .tags(vec!["important".to_string()])
            .limit(5);

        assert_eq!(query.text, "test query");
        assert_eq!(query.source, Some(MemorySource::Global));
        assert_eq!(query.tags, vec!["important"]);
        assert_eq!(query.limit, 5);
    }

    #[test]
    fn rank_results_works() {
        let entries = vec![
            MemoryEntry::new("important test content", MemorySource::Global)
                .with_importance(0.9),
            MemoryEntry::new("random content", MemorySource::Global),
            MemoryEntry::new("test", MemorySource::Global),
        ];

        let results = rank_results(entries, "test");
        assert_eq!(results.len(), 3);
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn test_extract_highlights() {
        let content = "This is a test content with test words.";
        let highlights = extract_highlights(content, "test", 10);

        assert!(!highlights.is_empty());
        assert!(highlights[0].contains("test"));
    }
}
