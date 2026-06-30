//! FFF indexer actor — owns the `SearchIndex` file index.
//!
//! This actor maintains a long-lived file index and answers search queries
//! from agent tools and the TUI via the event bus. It uses `ignore::WalkBuilder`
//! for gitignore-aware traversal, `sublime_fuzzy` for fuzzy ranking, and the
//! workspace `notify` 7.0 for file watching. LMDB/LMDB-free persistence is
//! handled via a JSON-based frecency store.
//!
//! ## Global State
//!
//! The actor registers its shared handles in a process-wide registry so that
//! tools can also access the index without going through the event bus.

use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub use self::ractor_fff_indexer::RactorFffIndexerActor;
pub use self::ractor_fff_indexer::RactorFffIndexerHandle;

mod ractor_fff_indexer;

// Re-export for use by other modules (e.g., file_pickers)
// format_git_status is defined in this file (see below).

#[cfg(test)]
mod tests;

/// Process-wide registry of the search index state.
///
/// This is an intentional service-locator for the long-lived search indexer. The
/// actor initializes it once at startup; after that it is read-mostly and all
/// mutations flow through the thread-safe `RwLock<Option<SearchIndexState>>`.
static SEARCH_INDEX_STATE: std::sync::OnceLock<Arc<RwLock<Option<SearchIndexStateInner>>>> =
    std::sync::OnceLock::new();

fn search_index_state() -> &'static Arc<RwLock<Option<SearchIndexStateInner>>> {
    SEARCH_INDEX_STATE.get_or_init(|| Arc::new(RwLock::new(None)))
}

/// The shared search index state owned by the indexer.
#[derive(Clone)]
pub struct FffSearchState {
    /// Filesystem root that was indexed.
    pub project_path: PathBuf,
    /// Search index (in-memory file map).
    pub index: SearchIndex,
    /// Index is ready.
    pub indexed: bool,
}

pub(crate) struct SearchIndexStateInner {
    state: FffSearchState,
}

impl FffSearchState {
    /// Attempt to read the current global search index state.
    ///
    /// Returns `None` if the indexer has not been spawned yet.
    pub fn get() -> Option<Self> {
        let guard = search_index_state().read();
        guard.as_ref().map(|inner| inner.state.clone())
    }

    /// Returns `true` if the global indexer has completed its initial scan.
    pub fn is_indexed() -> bool {
        let guard = search_index_state().read();
        guard.as_ref().map(|i| i.state.indexed).unwrap_or(false)
    }

    /// Reset the global search index state for test isolation.
    /// Clears the inner state so a new indexer can initialize with a fresh root.
    #[cfg(test)]
    pub fn reset_for_test() {
        *search_index_state().write() = None;
    }

    /// Record that a file was accessed (read or selected) to boost its frecency score.
    /// This is a best-effort operation — failures are silently ignored.
    pub fn record_file_access(&self, path: &std::path::Path) {
        if let Some(rel) = path.strip_prefix(&self.project_path).ok() {
            self.index.record_access(rel);
        }
    }
}

/// Default index scan timeout in seconds.
pub const SCAN_TIMEOUT_SECS: u64 = 30;

/// Default max search results per query.
pub const DEFAULT_LIMIT: usize = 50;

/// Event emitted by the FFF indexer on the bus after processing a search.
#[derive(Debug, Clone)]
pub struct FffSearchResult(pub FffSearchResultPayload);

/// Per-file search result item returned to callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FffFileItem {
    /// Relative path from the workspace root.
    pub relative_path: String,
    /// Absolute path on disk.
    pub absolute_path: String,
    /// Score from fuzzy matching (0-100).
    pub score: f64,
    /// Whether the file is tracked by git (not None).
    pub git_tracked: bool,
    /// Human-readable git status string ("modified", "untracked", "staged", etc.).
    pub git_status: Option<String>,
}

/// Search result emitted by the indexer on the event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FffSearchResultPayload {
    /// Correlates this result to its originating request.
    pub request_id: u64,
    /// The query that produced these results.
    pub query: String,
    /// Matched files.
    pub items: Vec<FffFileItem>,
    /// Total number of matches across all files.
    pub total_matched: usize,
    /// Whether the indexer has finished its initial scan.
    pub indexed: bool,
}

/// Message sent to the FFF indexer actor to trigger a search.
#[derive(Debug, Clone)]
pub struct FffSearchRequest {
    /// Opaque request ID for correlating results.
    pub request_id: u64,
    /// Query string.
    pub query: String,
    /// Maximum number of results to return.
    pub limit: Option<usize>,
    /// Workspace root path.
    pub project_path: PathBuf,
}

impl FffSearchRequest {
    /// Create a new search request.
    pub fn new(query: String, project_path: PathBuf) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static REQUEST_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            request_id: REQUEST_ID.fetch_add(1, Ordering::Relaxed),
            query,
            limit: Some(50),
            project_path,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SearchIndex — in-memory file map with fuzzy search and frecency
// ─────────────────────────────────────────────────────────────────────────────

use git2::Status as GitStatus;

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
    git_status: std::collections::HashMap<String, GitStatus>,
    indexed: bool,
}

/// Metadata for an indexed file.
#[derive(Debug, Clone)]
struct FileMetadata {
    /// Absolute path on disk.
    absolute_path: PathBuf,
    /// Is this a directory?
    #[allow(dead_code)]
    is_dir: bool,
}

impl SearchIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(parking_lot::RwLock::new(SearchIndexInner {
                files: std::collections::BTreeMap::new(),
                frecency: FrecencyStore::new(),
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
    pub fn record_access(&self, relative_path: &std::path::Path) {
        let mut inner = self.inner.write();
        inner.frecency.record(relative_path);
    }

    /// Build the index by walking `root` with gitignore support.
    pub fn build(&self, root: &std::path::Path) {
        use ignore::WalkBuilder;

        let mut inner = self.inner.write();

        // Walk the directory tree with gitignore support.
        for entry in WalkBuilder::new(root)
            .hidden(false) // Include hidden files but respect .gitignore
            .follow_links(false)
            .ignore(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build()
        {
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
        if let Some(repo) = git2::Repository::open(root).ok() {
            let mut opts = git2::StatusOptions::new();
            opts.include_untracked(true);
            opts.recurse_untracked_dirs(true);
            if let Ok(iter) = repo.statuses(Some(&mut opts)) {
                for entry in iter.iter() {
                    if let Some(path) = entry.path() {
                        let status = entry.status();
                        inner.git_status.insert(path.to_owned(), status);
                    }
                }
            }
        }

        inner.indexed = true;
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
                if parsed.git_status_filters().next().is_some() {
                    let status = inner.git_status.get(*path);
                    let matches = parsed.git_status_filters().all(|filter| {
                        status.map(|s| git_status_matches(*s, filter)).unwrap_or(false)
                    });
                    if !matches {
                        return false;
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
                        git_status: inner.git_status.get(path).copied(),
                    });
                }

                let fuzzy = sublime_fuzzy::FuzzySearch::new(text, &filename);
                let score = fuzzy.best_match().map(|m| m.score() as f64).unwrap_or(0.0);

                if score < 1.0 && text.len() > 2 {
                    // Try matching against full path as fallback.
                    let fuzzy_full = sublime_fuzzy::FuzzySearch::new(text, path);
                    let full_score =
                        fuzzy_full.best_match().map(|m| m.score() as f64).unwrap_or(0.0);
                    if full_score < 1.0 {
                        return None;
                    }
                    Some(FileSearchResult {
                        relative_path: path.clone(),
                        absolute_path: meta.absolute_path.clone(),
                        score: full_score,
                        git_status: inner.git_status.get(path).copied(),
                    })
                } else {
                    Some(FileSearchResult {
                        relative_path: path.clone(),
                        absolute_path: meta.absolute_path.clone(),
                        score,
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
                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
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
        let inner = self.inner.read();

        let glob = match globset::Glob::new(pattern) {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };

        inner
            .files
            .iter()
            .filter(|(path, _)| glob.compile_matcher().is_match(path))
            .map(|(path, meta)| {
                let git_status = inner.git_status.get(path).copied();
                FileSearchResult {
                    relative_path: path.clone(),
                    absolute_path: meta.absolute_path.clone(),
                    score: 0.0,
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

/// A fuzzy search result.
#[derive(Debug, Clone)]
pub struct FileSearchResult {
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub score: f64,
    pub git_status: Option<GitStatus>,
}

/// A content match from grep.
#[derive(Debug, Clone)]
pub struct ContentMatch {
    pub path: String,
    pub line_number: u64,
    pub col: usize,
    pub line_content: String,
    pub fuzzy_score: Option<i32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Frecency store — simple MRU-based scoring
// ─────────────────────────────────────────────────────────────────────────────

/// A simple MRU-based frecency store.
/// Each file gets a score = (access_count * recency_boost) where recency_boost
/// decays with time since last access.
#[derive(Debug)]
struct FrecencyStore {
    /// Map from relative path to (access_count, last_access_timestamp).
    accesses: std::collections::HashMap<String, (u32, u64)>,
}

impl FrecencyStore {
    fn new() -> Self {
        Self {
            accesses: std::collections::HashMap::new(),
        }
    }

    /// Record an access for the given path.
    fn record(&mut self, path: &std::path::Path) {
        let key = path.to_string_lossy().into_owned();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let entry = self.accesses.entry(key).or_insert((0, 0));
        entry.0 += 1; // Increment access count.
        entry.1 = now; // Update timestamp.
    }

    /// Get the frecency score for a path (0.0 if never accessed).
    fn score(&self, path: &str) -> f64 {
        let Some((count, last_access)) = self.accesses.get(path) else {
            return 0.0;
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Decay: score halves every hour.
        let hours = (now.saturating_sub(*last_access)) as f64 / 3600.0;
        let decay = 0.5f64.powf(hours);

        *count as f64 * decay
    }
}

impl Default for FrecencyStore {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Content search helpers
// ─────────────────────────────────────────────────────────────────────────────

fn find_regex_matches(
    content: &str,
    re: &regex::Regex,
    max_per_file: usize,
    path: &str,
) -> Vec<ContentMatch> {
    let mut matches = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if matches.len() >= max_per_file {
            break;
        }
        if let Some(m) = re.find(line) {
            let score = sublime_fuzzy::FuzzySearch::new(path, line)
                .best_match()
                .map(|r| r.score() as i32);
            matches.push(ContentMatch {
                path: path.to_owned(),
                line_number: (line_num + 1) as u64,
                col: m.start() + 1,
                line_content: line.to_owned(),
                fuzzy_score: score,
            });
        }
    }
    matches
}

fn find_literal_matches(
    content: &str,
    query: &str,
    max_per_file: usize,
    path: &str,
) -> Vec<ContentMatch> {
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if matches.len() >= max_per_file {
            break;
        }
        if line.to_lowercase().contains(&query_lower) {
            let col = line.to_lowercase().find(&query_lower).unwrap_or(0) + 1;
            let score = sublime_fuzzy::FuzzySearch::new(path, line)
                .best_match()
                .map(|r| r.score() as i32);
            matches.push(ContentMatch {
                path: path.to_owned(),
                line_number: (line_num + 1) as u64,
                col,
                line_content: line.to_owned(),
                fuzzy_score: score,
            });
        }
    }
    matches
}

/// Max file size for content indexing (in bytes).
/// Matches fff_search::MAX_FFFILE_SIZE behavior.
pub const MAX_FILE_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

// ─────────────────────────────────────────────────────────────────────────────
// Git status helpers
// ─────────────────────────────────────────────────────────────────────────────

fn git_status_matches(status: GitStatus, filter: &str) -> bool {
    match filter {
        "modified" => {
            status.contains(GitStatus::WT_MODIFIED) || status.contains(GitStatus::INDEX_MODIFIED)
        }
        "untracked" => status.contains(GitStatus::WT_NEW) || status.contains(GitStatus::INDEX_NEW),
        "deleted" => {
            status.contains(GitStatus::WT_DELETED) || status.contains(GitStatus::INDEX_DELETED)
        }
        "renamed" => {
            status.contains(GitStatus::WT_RENAMED) || status.contains(GitStatus::INDEX_RENAMED)
        }
        "staged" => {
            status.contains(GitStatus::INDEX_MODIFIED)
                || status.contains(GitStatus::INDEX_NEW)
                || status.contains(GitStatus::INDEX_DELETED)
                || status.contains(GitStatus::INDEX_RENAMED)
        }
        "clean" => status.is_empty(),
        _ => false,
    }
}

/// Format a git2 Status as a human-readable label string.
///
/// Priority order mirrors `git_status_matches`: staged flags take precedence,
/// then unstaged, then renamed/deleted, then "untracked". Returns `"clean"`
/// when no tracked-change flags are set.
pub fn format_git_status(status: git2::Status) -> &'static str {
    // Staged flags — most important for display.
    if status.contains(GitStatus::INDEX_MODIFIED)
        || status.contains(GitStatus::INDEX_NEW)
        || status.contains(GitStatus::INDEX_DELETED)
        || status.contains(GitStatus::INDEX_RENAMED)
    {
        return if status.contains(GitStatus::INDEX_DELETED) {
            "deleted"
        } else if status.contains(GitStatus::INDEX_RENAMED) {
            "renamed"
        } else if status.contains(GitStatus::INDEX_NEW) {
            "untracked"
        } else {
            "modified"
        };
    }
    // Unstaged flags.
    if status.contains(GitStatus::WT_MODIFIED)
        || status.contains(GitStatus::WT_NEW)
        || status.contains(GitStatus::WT_DELETED)
        || status.contains(GitStatus::WT_RENAMED)
    {
        return if status.contains(GitStatus::WT_DELETED) {
            "deleted"
        } else if status.contains(GitStatus::WT_RENAMED) {
            "renamed"
        } else if status.contains(GitStatus::WT_NEW) {
            "untracked"
        } else {
            "modified"
        };
    }
    "clean"
}
