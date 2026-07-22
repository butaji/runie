//! Memory search index implementation.
//!
//! Provides in-memory keyword-based search with relevance ranking.
//! Uses TF-IDF-like scoring for keyword matching.
//!
//! For production with vector embeddings, this can be extended to use
//! SQLite FTS5 or an external vector database.

use crate::memory::storage::{MemoryEntry, MemoryScope, MemoryStorage};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// A indexed document with metadata.
#[derive(Debug, Clone)]
pub struct IndexedDocument {
    /// Document ID.
    pub id: String,
    /// Document content.
    pub content: String,
    /// Pre-computed tokens for search.
    tokens: HashSet<String>,
    /// Token frequencies.
    token_freqs: HashMap<String, usize>,
    /// Scope of the memory.
    pub scope: MemoryScope,
    /// Workspace hash if applicable.
    pub workspace: Option<String>,
    /// Importance score.
    pub importance: f32,
    /// Timestamp for recency scoring.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl IndexedDocument {
    /// Create from a memory entry.
    pub fn from_entry(entry: &MemoryEntry) -> Self {
        let id = entry.id().to_string();
        let content = entry.content.clone();
        let tokens = tokenize(&content);
        let token_freqs = compute_frequencies(&tokens);
        let scope = entry.frontmatter.scope;
        let workspace = entry.frontmatter.workspace.clone();
        let importance = entry.frontmatter.importance;
        let timestamp = entry.frontmatter.timestamp;

        Self {
            id,
            content,
            tokens,
            token_freqs,
            scope,
            workspace,
            importance,
            timestamp,
        }
    }
}

/// Tokenize text into searchable terms.
pub fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

/// Compute term frequencies from tokens.
fn compute_frequencies(tokens: &HashSet<String>) -> HashMap<String, usize> {
    // For now, just use 1 for each token since we're working with sets
    // A more sophisticated version would track actual frequencies
    tokens.iter().map(|t| (t.clone(), 1)).collect()
}

/// A search result with relevance score.
#[derive(Debug, Clone)]
pub struct IndexedResult {
    /// Document ID.
    pub id: String,
    /// Document content.
    pub content: String,
    /// Relevance score (0.0 to 1.0).
    pub score: f32,
    /// Scope of the matched document.
    pub scope: MemoryScope,
    /// Workspace hash if applicable.
    pub workspace: Option<String>,
    /// Match highlights.
    pub highlights: Vec<String>,
}

impl IndexedResult {
    /// Create a new indexed result.
    pub fn new(doc: &IndexedDocument, score: f32) -> Self {
        Self {
            id: doc.id.clone(),
            content: doc.content.clone(),
            score,
            scope: doc.scope,
            workspace: doc.workspace.clone(),
            highlights: Vec::new(),
        }
    }

    /// Add a highlight.
    pub fn with_highlight(mut self, highlight: String) -> Self {
        self.highlights.push(highlight);
        self
    }
}

/// In-memory search index for memory entries.
#[derive(Debug, Default)]
pub struct MemoryIndex {
    /// All indexed documents.
    documents: Vec<IndexedDocument>,
    /// Document lookup by ID.
    doc_by_id: HashMap<String, usize>,
    /// Global document frequency (for IDF calculation).
    global_freq: HashMap<String, usize>,
}

impl MemoryIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            doc_by_id: HashMap::new(),
            global_freq: HashMap::new(),
        }
    }

    /// Index a single document.
    pub fn add(&mut self, doc: IndexedDocument) {
        let id = doc.id.clone();

        // Update global frequency
        for token in &doc.tokens {
            *self.global_freq.entry(token.clone()).or_insert(0) += 1;
        }

        // Store document
        let idx = self.documents.len();
        self.documents.push(doc);
        self.doc_by_id.insert(id, idx);
    }

    /// Index a memory entry.
    pub fn add_entry(&mut self, entry: &MemoryEntry) {
        let doc = IndexedDocument::from_entry(entry);
        self.add(doc);
    }

    /// Index multiple entries.
    pub fn add_entries(&mut self, entries: &[MemoryEntry]) {
        for entry in entries {
            self.add_entry(entry);
        }
    }

    /// Rebuild index from storage.
    pub fn rebuild_from_storage(storage: &MemoryStorage, workspace_hash: Option<&str>) -> Result<Self> {
        let mut index = Self::new();

        // Load global memories
        if let Ok(entries) = storage.read_global_memory() {
            index.add_entries(&entries);
        }

        // Load workspace memories if provided
        if let Some(hash) = workspace_hash {
            if let Ok(entries) = storage.read_workspace_memory(hash) {
                index.add_entries(&entries);
            }
            if let Ok(entries) = storage.read_sessions(hash) {
                index.add_entries(&entries);
            }
        }

        Ok(index)
    }

    /// Search the index for a query.
    pub fn search(&self, query: &str, limit: usize) -> Vec<IndexedResult> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<IndexedResult> = self
            .documents
            .iter()
            .map(|doc| {
                let score = self.compute_score(&query_tokens, doc);
                IndexedResult::new(doc, score)
            })
            .filter(|r| r.score > 0.0)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Add highlights
        for result in &mut results {
            result.highlights = extract_highlights(&result.content, query, 50);
        }

        results.truncate(limit);
        results
    }

    /// Compute relevance score for a document given query tokens.
    fn compute_score(&self, query_tokens: &HashSet<String>, doc: &IndexedDocument) -> f32 {
        if query_tokens.is_empty() {
            return 0.0;
        }

        let mut score = 0.0f32;

        // Term frequency component (TF)
        let matched_tokens: Vec<_> = query_tokens
            .intersection(&doc.tokens)
            .collect();

        if matched_tokens.is_empty() {
            return 0.0;
        }

        // TF-IDF scoring
        let n_docs = self.documents.len().max(1) as f32;
        for token in matched_tokens {
            let token_str: &str = token;
            let tf = *doc.token_freqs.get(token_str).unwrap_or(&1) as f32;
            let df = *self.global_freq.get(token_str).unwrap_or(&1) as f32;
            let idf = (n_docs / df).ln() + 1.0;
            score += tf * idf;
        }

        // Normalize by query length
        score /= query_tokens.len() as f32;

        // Boost by importance
        score += doc.importance * 0.1;

        // Recency boost (exponential decay over 30 days)
        let days_old = (chrono::Utc::now() - doc.timestamp).num_days() as f32;
        let recency = (-days_old / 30.0).exp();
        score += recency * 0.05;

        // Scope boost (prefer workspace over session)
        match doc.scope {
            MemoryScope::Workspace => score *= 1.2,
            MemoryScope::Session => score *= 0.8,
            MemoryScope::Global => {} // No modification
        }

        score.clamp(0.0, 1.0)
    }

    /// Get a document by ID.
    pub fn get(&self, id: &str) -> Option<&IndexedDocument> {
        self.doc_by_id.get(id).and_then(|&idx| self.documents.get(idx))
    }

    /// Get total document count.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if index is empty.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Clear the index.
    pub fn clear(&mut self) {
        self.documents.clear();
        self.doc_by_id.clear();
        self.global_freq.clear();
    }
}

impl Default for IndexedDocument {
    fn default() -> Self {
        Self {
            id: String::new(),
            content: String::new(),
            tokens: HashSet::new(),
            token_freqs: HashMap::new(),
            scope: MemoryScope::Workspace,
            workspace: None,
            importance: 0.5,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Extract highlighted snippets from content.
fn extract_highlights(content: &str, query: &str, context_chars: usize) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let content_lower = content.to_lowercase();
    let mut highlights = Vec::new();

    let mut search_start = 0;
    while let Some(pos) = content_lower[search_start..].find(&query_lower) {
        let abs_pos = search_start + pos;
        let ctx_start = abs_pos.saturating_sub(context_chars);
        let ctx_end = (abs_pos + query.len() + context_chars).min(content.len());

        let mut highlight = String::new();
        if ctx_start > 0 {
            highlight.push_str("...");
        }
        highlight.push_str(&content[ctx_start..ctx_end]);
        if ctx_end < content.len() {
            highlight.push_str("...");
        }

        highlights.push(highlight);

        search_start = abs_pos + query.len();
        if highlights.len() >= 3 {
            break;
        }
    }

    highlights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello World! This is a test.");
        #[allow(clippy::unnecessary_to_owned)]
        {
            assert!(tokens.contains(&"hello".to_string()));
            assert!(tokens.contains(&"world".to_string()));
            assert!(tokens.contains(&"this".to_string()));
            assert!(!tokens.contains(&"a".to_string())); // Too short
        }
    }

    #[test]
    fn test_index_search() {
        let mut index = MemoryIndex::new();

        let entry1 = MemoryEntry::new("Rust programming language is great", MemoryScope::Workspace)
            .with_importance(0.9);
        let entry2 = MemoryEntry::new("Python is also popular", MemoryScope::Global);
        let entry3 = MemoryEntry::new("JavaScript for web development", MemoryScope::Workspace);

        index.add_entry(&entry1);
        index.add_entry(&entry2);
        index.add_entry(&entry3);

        let results = index.search("rust programming", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, entry1.id());
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_scope_scoring() {
        let mut index = MemoryIndex::new();

        let ws_entry = MemoryEntry::new("workspace test content", MemoryScope::Workspace);
        let session_entry = MemoryEntry::new("session test content", MemoryScope::Session);

        index.add_entry(&ws_entry);
        index.add_entry(&session_entry);

        let results = index.search("test", 10);
        assert!(results.len() >= 2);
        // Workspace should score higher
        let ws_result = results.iter().find(|r| r.scope == MemoryScope::Workspace).unwrap();
        let session_result = results.iter().find(|r| r.scope == MemoryScope::Session).unwrap();
        assert!(ws_result.score >= session_result.score);
    }

    #[test]
    fn test_empty_query() {
        let index = MemoryIndex::new();
        let results = index.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_extract_highlights() {
        let content = "This is a long test content with some test keywords.";
        let highlights = extract_highlights(content, "test", 10);
        assert!(!highlights.is_empty());
        assert!(highlights[0].contains("test"));
    }
}
