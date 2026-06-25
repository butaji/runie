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

use crate::actors::{Actor, ActorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::FffFileEntry;
use fff_search::{SharedFilePicker, SharedFrecency, SharedQueryTracker};
use std::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

mod search;
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

// ─────────────────────────────────────────────────────────────────────────────
// FffIndexerActor
// ─────────────────────────────────────────────────────────────────────────────

/// Long-lived actor that owns the FFF file index.
///
/// `FffIndexerActor` is spawned once per workspace session. It:
/// - Initializes the FFF `FilePicker` on startup and waits for the initial scan.
/// - Handles `FffSearchRequest` messages by querying the shared picker.
/// - Emits `FffSearchResult` events back on the bus.
pub struct FffIndexerActor {
    /// Filesystem root to index.
    root: PathBuf,
    /// LMDB path for frecency data (under `~/.cache/runie/fff/`).
    frecency_path: PathBuf,
    /// LMDB path for query tracker data.
    query_path: PathBuf,
    /// Shared FFF picker handle.
    shared_picker: SharedFilePicker,
    /// Shared frecency tracker.
    shared_frecency: SharedFrecency,
    /// Shared query tracker.
    shared_query_tracker: SharedQueryTracker,
    /// Whether the initial scan has completed.
    indexed: bool,
}

impl FffIndexerActor {
    /// Construct a new actor for the given workspace root.
    pub fn new(root: PathBuf, data_dir: PathBuf) -> anyhow::Result<Self> {
        let fff_dir = data_dir.join("runie").join("fff");
        Ok(Self {
            root,
            frecency_path: fff_dir.join("frecency"),
            query_path: fff_dir.join("queries"),
            shared_picker: SharedFilePicker::default(),
            shared_frecency: SharedFrecency::default(),
            shared_query_tracker: SharedQueryTracker::default(),
            indexed: false,
        })
    }

    /// Spawn the indexer actor and return a handle + bus tx.
    ///
    /// Returns `(tx, actor_handle)` — send `FffSearchRequest` via `tx`.
    /// The actor emits `Event::FffSearchResult` events on `bus`.
    pub fn spawn(
        root: PathBuf,
        data_dir: PathBuf,
        bus: EventBus<Event>,
    ) -> anyhow::Result<(mpsc::Sender<FffSearchRequest>, ActorHandle)> {
        let actor = Self::new(root, data_dir)?;
        let (tx, rx) = mpsc::channel(64);
        let handle = ActorHandle::spawn(actor, rx, bus);
        Ok((tx, handle))
    }
}

impl Actor for FffIndexerActor {
    type Msg = FffSearchRequest;
    type Event = Event;

    async fn run_body(self, rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Event>) {
        self.run_inner(rx, bus).await;
    }
}

impl FffIndexerActor {
    /// Main event loop.
    async fn run_inner(
        mut self,
        mut rx: mpsc::Receiver<FffSearchRequest>,
        bus: EventBus<Event>,
    ) {
        // Initialize FFF shared state
        if let Err(e) = self.init_fff().await {
            tracing::error!("fff indexer init failed: {e}");
            return;
        }

        // Process messages until the channel closes
        while let Some(request) = rx.recv().await {
            let payload = self.handle_search(request).await;
            let entries: Vec<FffFileEntry> = payload
                .items
                .iter()
                .map(|item| FffFileEntry {
                    name: std::path::Path::new(&item.relative_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| item.relative_path.clone()),
                    path: item.relative_path.clone(),
                    is_dir: item.relative_path.ends_with('/'),
                    score: item.score,
                    git_status: item.git_status.clone(),
                })
                .collect();
            bus.publish(Event::FffSearchResult {
                request_id: payload.request_id,
                entries,
                query: payload.query,
                indexed: payload.indexed,
            });
        }
    }
}
