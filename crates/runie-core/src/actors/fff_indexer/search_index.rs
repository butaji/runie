//! FFF search index — in-memory file map with fuzzy search.
//!
//! Provides the `SearchIndex` struct which maintains an in-memory map of files
//! with fuzzy ranking and git status tracking.

use std::path::Path;

#[cfg(feature = "git")]
use git2::Status as GitStatus;

use super::{ContentMatch, FileSearchResult};
use crate::actors::fff_indexer::content_search::{find_literal_matches, find_regex_matches};
use crate::actors::fff_indexer::frecency::FrecencyStore;
#[cfg(feature = "git")]
#[allow(unused_imports)]
use crate::actors::fff_indexer::git_status::{format_git_status, git_status_matches};

/// Metadata for an indexed file.
#[derive(Debug, Clone)]
pub(super) struct FileMetadata {
    /// Absolute path on disk.
    pub(super) absolute_path: std::path::PathBuf,
    /// Is this a directory?
    #[allow(dead_code)]
    pub(super) is_dir: bool,
}

/// In-memory file index with fuzzy search and frecency ranking.
#[derive(Clone)]
pub struct SearchIndex {
    inner: Arc<parking_lot::RwLock<SearchIndexInner>>,
}

struct SearchIndexInner {
    /// All indexed files (relative path -> metadata).
    files: std::collections::BTreeMap<String, FileMetadata>,
    /// Frecency scores (relative path -> access count + recency decay).
    frecency: FrecencyStore,
    /// Git status for each file.
    #[cfg(feature = "git")]
    git_status: std::collections::HashMap<String, GitStatus>,
    indexed: bool,
}

impl SearchIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(parking_lot::RwLock::new(SearchIndexInner {
                files: std::collections::BTreeMap::new(),
                frecency: FrecencyStore::new(),
                #[cfg(feature = "git")]
                git_status: std::collections::HashMap::new(),
                indexed: false,
            })),
        }
    }

    /// Returns true if the index has completed its initial scan.
    pub fn is_indexed(&self) -> bool {
        self.inner.read().indexed
    }

    /// Record that a file at `relative_path` was accessed.
    pub fn record_access(&self, relative_path: &Path) {
        let mut inner = self.inner.write();
        inner.frecency.record(relative_path);
    }

    /// Build the index by walking `root` with gitignore support.
    ///
    /// The scan respects [`super::is_indexer_scan_cancelled`] so it stops early
    /// when the user quits while indexing is still running, letting the process
    /// exit immediately.
    pub fn build(&self, root: &Path) {
        use ignore::WalkBuilder;

        let mut inner = self.inner.write();

        // Walk the directory tree with gitignore support. `.hidden(false)`
        // includes dotfiles, so `.git` must be pruned explicitly — VCS
        // internals are never useful as picker candidates.
        for entry in WalkBuilder::new(root)
            .hidden(false) // Include hidden files but respect .gitignore
            .follow_links(false)
            .ignore(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .filter_entry(|e| e.file_name() != ".git")
            .build()
        {
            if super::is_indexer_scan_cancelled() {
                break;
            }
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    let rel = match path.strip_prefix(root) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };
                    let rel_str = rel.to_string_lossy().into_owned();

                    // Skip directories themselves in the map (they're implied by file paths).
                    if path.is_dir() {
                        continue;
                    }

                    inner.files.insert(
                        rel_str.clone(),
                        FileMetadata {
                            absolute_path: path.to_path_buf(),
                            is_dir: false,
                        },
                    );
                }
                Err(e) => {
                    tracing::debug!("indexer: walk error: {e}");
                }
            }
        }

        // Get git status for all files.
        #[cfg(feature = "git")]
        {
            if !super::is_indexer_scan_cancelled() {
                if let Ok(repo) = git2::Repository::open(root) {
                    let mut opts = git2::StatusOptions::new();
                    opts.include_untracked(true);
                    opts.recurse_untracked_dirs(true);
                    if let Ok(iter) = repo.statuses(Some(&mut opts)) {
                        for entry in iter.iter() {
                            if super::is_indexer_scan_cancelled() {
                                break;
                            }
                            if let Some(path) = entry.path() {
                                let status = entry.status();
                                inner.git_status.insert(path.to_owned(), status);
                            }
                        }
                    }
                }
            }
        }

        inner.indexed = !super::is_indexer_scan_cancelled();
    }

    /// Perform a fuzzy file search and return scored results.
    pub fn fuzzy_search(&self, query: &str, limit: usize) -> Vec<FileSearchResult> {
        let inner = self.inner.read();

        // Parse query to extract constraints.
        let parsed = crate::location::parse_search_query(query);
        let text = parsed.text.trim();

        let mut scored: Vec<_> = inner
            .files
            .iter()
            .filter(|(path, _meta)| {
                // Apply path negations.
                if parsed.negations().any(|n| path.contains(n)) {
                    return false;
                }

                // Apply glob constraints.
                if parsed.globs().next().is_some() {
                    use globset::Glob;
                    for glob_pat in parsed.globs() {
                        let glob = match Glob::new(glob_pat) {
                            Ok(g) => g,
                            Err(_) => continue,
                        };
                        if !glob.compile_matcher().is_match(path) {
                            return false;
                        }
                    }
                }

                // Apply git-status filters.
                #[cfg(feature = "git")]
                {
                    if parsed.git_status_filters().next().is_some() {
                        let status = inner.git_status.get(*path);
                        let matches = parsed.git_status_filters().all(|filter| {
                            status
                                .map(|s| git_status_matches(*s, filter))
                                .unwrap_or(false)
                        });
                        if !matches {
                            return false;
                        }
                    }
                }

                true
            })
            .filter_map(|(path, meta)| {
                // Fuzzy match against the filename (not full path).
                let filename = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone());

                if text.is_empty() {
                    return Some(FileSearchResult {
                        relative_path: path.clone(),
                        absolute_path: meta.absolute_path.clone(),
                        score: 0.0,
                        #[cfg(feature = "git")]
                        git_status: inner.git_status.get(path).copied(),
                    });
                }

                let fuzzy = sublime_fuzzy::FuzzySearch::new(text, &filename);
                let score = fuzzy.best_match().map(|m| m.score() as f64).unwrap_or(0.0);

                if score < 1.0 && text.len() > 2 {
                    // Try matching against full path as fallback.
                    let fuzzy_full = sublime_fuzzy::FuzzySearch::new(text, path);
                    let full_score = fuzzy_full
                        .best_match()
                        .map(|m| m.score() as f64)
                        .unwrap_or(0.0);
                    if full_score < 1.0 {
                        return None;
                    }
                    Some(FileSearchResult {
                        relative_path: path.clone(),
                        absolute_path: meta.absolute_path.clone(),
                        score: full_score,
                        #[cfg(feature = "git")]
                        git_status: inner.git_status.get(path).copied(),
                    })
                } else {
                    Some(FileSearchResult {
                        relative_path: path.clone(),
                        absolute_path: meta.absolute_path.clone(),
                        score,
                        #[cfg(feature = "git")]
                        git_status: inner.git_status.get(path).copied(),
                    })
                }
            })
            .collect();

        // Sort by score descending, then by frecency.
        {
            let frecency = &inner.frecency;
            scored.sort_by(|a, b| {
                let score_a = a.score + frecency.score(&a.relative_path);
                let score_b = b.score + frecency.score(&b.relative_path);
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        scored.truncate(limit);
        scored
    }

    /// Perform a content search (grep) across indexed files.
    pub fn grep(
        &self,
        query: &str,
        max_file_size: usize,
        max_matches_per_file: usize,
        limit: usize,
    ) -> Vec<ContentMatch> {
        let inner = self.inner.read();

        // Try to compile as regex; fall back to literal search.
        let regex = regex::RegexBuilder::new(query)
            .case_insensitive(true)
            .build()
            .ok();

        let mut matches = Vec::new();

        for (path, meta) in &inner.files {
            // Skip files that are too large.
            if let Ok(metadata) = std::fs::metadata(&meta.absolute_path) {
                if metadata.len() as usize > max_file_size {
                    continue;
                }
            }

            let content = match std::fs::read_to_string(&meta.absolute_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let file_matches = if let Some(ref re) = regex {
                find_regex_matches(&content, re, max_matches_per_file, path)
            } else {
                find_literal_matches(&content, query, max_matches_per_file, path)
            };

            for m in file_matches {
                if matches.len() >= limit {
                    return matches;
                }
                matches.push(m);
            }
        }

        matches
    }

    /// Perform a glob search.
    pub fn glob_search(&self, pattern: &str, limit: usize) -> Vec<FileSearchResult> {
        use globset::Glob;

        let inner = self.inner.read();

        let glob = match Glob::new(pattern) {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };

        inner
            .files
            .iter()
            .filter(|(path, _)| glob.compile_matcher().is_match(path))
            .map(|(path, meta)| {
                #[cfg(feature = "git")]
                let git_status = inner.git_status.get(path).copied();
                FileSearchResult {
                    relative_path: path.clone(),
                    absolute_path: meta.absolute_path.clone(),
                    score: 0.0,
                    #[cfg(feature = "git")]
                    git_status,
                }
            })
            .take(limit)
            .collect()
    }

    /// Total number of indexed files.
    pub fn file_count(&self) -> usize {
        self.inner.read().files.len()
    }
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

// Required for tests
use std::sync::Arc;
