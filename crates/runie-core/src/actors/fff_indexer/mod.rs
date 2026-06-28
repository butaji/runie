//! FFF indexer actor — owns the `fff-search` file index.
//!
//! This actor maintains a long-lived file index and answers search queries
//! from agent tools and the TUI via the event bus. It uses the `fff-search`
//! crate's `SharedFilePicker` / `SharedFrecency` / `SharedQueryTracker` handles
//! for safe concurrent access from multiple tokio tasks.
//!
//! ## Global State
//!
//! The actor registers its shared handles in a process-wide registry so that
//! tools can also access the index without going through the event bus.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use fff_search::{SharedFilePicker, SharedFrecency, SharedQueryTracker};

pub use self::ractor_fff_indexer::RactorFffIndexerHandle;
pub use self::ractor_fff_indexer::RactorFffIndexerActor;

mod ractor_fff_indexer;

// Re-export for use by other modules (e.g., file_pickers)
pub use ractor_fff_indexer::format_git_status;

#[cfg(test)]
mod tests;

/// Process-wide registry of FFF shared state.
///
/// This is an intentional service-locator for the long-lived FFF indexer. The
/// actor initializes it once at startup; after that it is read-mostly and all
/// mutations flow through the thread-safe `SharedFilePicker` / `SharedFrecency`
/// handles it contains. A fully actorised request/reply design would require
/// threading a sender through every tool, so this locator keeps the 80 % case
/// simple while the actor still owns the underlying shared handles.
static FFF_STATE: std::sync::OnceLock<Arc<RwLock<Option<FffSearchStateInner>>>> =
    std::sync::OnceLock::new();

fn fff_state() -> &'static Arc<RwLock<Option<FffSearchStateInner>>> {
    FFF_STATE.get_or_init(|| Arc::new(RwLock::new(None)))
}

/// The actual shared state handles owned by the indexer.
#[derive(Clone)]
pub struct FffSearchState {
    /// Filesystem root that was indexed.
    pub project_path: PathBuf,
    /// Shared FFF file picker.
    pub picker: SharedFilePicker,
    /// Shared frecency tracker.
    pub frecency: SharedFrecency,
    /// Shared query tracker.
    pub query_tracker: SharedQueryTracker,
}

struct FffSearchStateInner {
    state: FffSearchState,
    indexed: bool,
}

impl FffSearchState {
    /// Attempt to read the current global FFF state.
    ///
    /// Returns `None` if the indexer has not been spawned yet.
    pub fn get() -> Option<Self> {
        let guard = fff_state().read().ok()?;
        guard.as_ref().map(|inner| inner.state.clone())
    }

    /// Returns `true` if the global indexer has completed its initial scan.
    pub fn is_indexed() -> bool {
        let guard = match fff_state().read() {
            Ok(g) => g,
            Err(_) => return false,
        };
        guard.as_ref().map(|i| i.indexed).unwrap_or(false)
    }

    /// Reset the global FFF state for test isolation.
    /// Clears the inner state so a new indexer can initialize with a fresh root.
    #[cfg(test)]
    pub fn reset_for_test() {
        if let Ok(mut g) = fff_state().write() {
            *g = None;
        }
    }

    /// Record that a file was accessed (read or selected) to boost its frecency score.
    /// This is a best-effort operation — failures are silently ignored.
    pub fn record_file_access(&self, path: &std::path::Path) {
        // Must use write guard for picker since update_single_file_frecency needs &mut.
        let mut picker_guard = match self.picker.write() {
            Ok(g) => g,
            Err(_) => return,
        };
        let frecency_guard = match self.frecency.read() {
            Ok(g) => g,
            Err(_) => return,
        };

        if let (Some(picker), Some(frecency)) = (picker_guard.as_mut(), frecency_guard.as_ref()) {
            let _ = picker.update_single_file_frecency(path, frecency);
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
    /// FFF's total relevance score.
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
    /// Query string (fff-query-parser syntax).
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


