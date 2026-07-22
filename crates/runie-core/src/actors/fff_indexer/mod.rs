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

mod content_search;
mod frecency;
#[cfg(feature = "git")]
mod git_status;
mod search_index;

use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Global flag used to abort the startup index scan as soon as the user quits.
/// The scan runs on a blocking thread, so a shared atomic is the simplest
/// cross-layer cancellation signal.
static INDEXER_CANCEL: AtomicBool = AtomicBool::new(false);

/// Ask the background indexer scan to stop. Safe to call multiple times.
pub fn cancel_indexer_scan() {
    INDEXER_CANCEL.store(true, Ordering::Release);
}

/// Returns true if the indexer scan has been cancelled.
pub fn is_indexer_scan_cancelled() -> bool {
    INDEXER_CANCEL.load(Ordering::Acquire)
}

pub use self::ractor_fff_indexer::RactorFffIndexerActor;
pub use self::ractor_fff_indexer::RactorFffIndexerHandle;

mod ractor_fff_indexer;

// Re-exports for use by other modules (e.g., file_pickers)
#[cfg(feature = "git")]
pub use git_status::format_git_status;
pub use search_index::SearchIndex;

#[cfg(test)]
mod tests;

// ============================================================================
// Types — shared state, requests, results
// ============================================================================

/// Process-wide registry of the search index state.
///
/// This is an intentional service-locator for the long-lived search indexer. The
/// actor initializes it once at startup; after that it is read-mostly and all
/// mutations flow through the thread-safe `RwLock<Option<SearchIndexState>>`.
static SEARCH_INDEX_STATE: std::sync::OnceLock<Arc<RwLock<Option<SearchIndexStateInner>>>> = std::sync::OnceLock::new();

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
        if let Ok(rel) = path.strip_prefix(&self.project_path) {
            self.index.record_access(rel);
        }
    }
}

// ── Constants ────────────────────────────────────────────────────────────────

/// Default index scan timeout in seconds.
pub const SCAN_TIMEOUT_SECS: u64 = 30;

/// Default max search results per query.
pub const DEFAULT_LIMIT: usize = 50;

/// Max file size for content indexing (in bytes).
pub const MAX_FILE_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

// ── Result types ─────────────────────────────────────────────────────────────

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
        Self { request_id: REQUEST_ID.fetch_add(1, Ordering::Relaxed), query, limit: Some(50), project_path }
    }
}

// ── Search result types ───────────────────────────────────────────────────────

/// A fuzzy search result.
#[derive(Debug, Clone)]
pub struct FileSearchResult {
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub score: f64,
    /// Git status for the file, if available (requires `git` feature).
    #[cfg(feature = "git")]
    pub git_status: Option<git2::Status>,
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
