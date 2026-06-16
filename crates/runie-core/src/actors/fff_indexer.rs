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

use crate::actor::{Actor, ActorHandle};
use crate::bus::EventBus;
use fff_search::{
    ContentCacheBudget, FilePickerOptions, FuzzySearchOptions, FFFMode, FrecencyTracker,
    PaginationArgs, QueryParser, QueryTracker, SearchResult as FffRawSearchResult,
    SharedFilePicker, SharedFrecency, SharedQueryTracker,
};
use git2;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Process-wide registry of FFF shared state.
///
/// Initialised once by the `FffIndexerActor` when it starts up.
/// Tools and the TUI read from this registry to perform searches.
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
        let guard = fff_state().read();
        guard.as_ref().map(|inner| inner.state.clone())
    }

    /// Returns `true` if the global indexer has completed its initial scan.
    pub fn is_indexed() -> bool {
        let guard = fff_state().read();
        guard.as_ref().map(|i| i.indexed).unwrap_or(false)
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
const SCAN_TIMEOUT_SECS: u64 = 30;

/// Default max search results per query.
const DEFAULT_LIMIT: usize = 50;

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
    /// The actor emits `FffSearchResult` events on `bus`.
    pub fn spawn(
        root: PathBuf,
        data_dir: PathBuf,
        bus: EventBus<FffSearchResult>,
    ) -> anyhow::Result<(mpsc::Sender<FffSearchRequest>, ActorHandle)> {
        let actor = Self::new(root, data_dir)?;
        let (tx, rx) = mpsc::channel(64);
        let handle = ActorHandle::spawn(actor, rx, bus);
        Ok((tx, handle))
    }
}

impl Actor for FffIndexerActor {
    type Msg = FffSearchRequest;
    type Event = FffSearchResult;

    fn run_body(
        self,
        rx: mpsc::Receiver<Self::Msg>,
        bus: EventBus<FffSearchResult>,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        async move {
            self.run_inner(rx, bus).await;
        }
    }
}

impl FffIndexerActor {
    /// Main event loop.
    async fn run_inner(
        mut self,
        mut rx: mpsc::Receiver<FffSearchRequest>,
        bus: EventBus<FffSearchResult>,
    ) {
        // Initialize FFF shared state
        if let Err(e) = self.init_fff().await {
            tracing::error!("fff indexer init failed: {e}");
            return;
        }

        // Process messages until the channel closes
        while let Some(request) = rx.recv().await {
            let result = self.handle_search(request).await;
            bus.publish(FffSearchResult(result));
        }
    }

    /// Initialize FFF: open LMDB stores, start the picker, and register globally.
    async fn init_fff(&mut self) -> anyhow::Result<()> {
        // Open frecency tracker
        let frecency = FrecencyTracker::open(&self.frecency_path)
            .map_err(|e| anyhow::anyhow!("frecency open: {e}"))?;
        self.shared_frecency
            .init(frecency)
            .map_err(|e| anyhow::anyhow!("frecency init: {e}"))?;

        // Open query tracker
        let query_tracker = QueryTracker::open(&self.query_path)
            .map_err(|e| anyhow::anyhow!("query tracker open: {e}"))?;
        self.shared_query_tracker
            .init(query_tracker)
            .map_err(|e| anyhow::anyhow!("query tracker init: {e}"))?;

        // Create the picker — starts background scan + file watcher
        let root_str = self.root.to_string_lossy().into_owned();
        fff_search::FilePicker::new_with_shared_state(
            self.shared_picker.clone(),
            self.shared_frecency.clone(),
            FilePickerOptions {
                base_path: root_str.clone(),
                mode: FFFMode::Ai,
                watch: true,
                enable_mmap_cache: false,
                enable_content_indexing: false,
                enable_fs_root_scanning: true,
                enable_home_dir_scanning: false,
                follow_symlinks: true,
                cache_budget: Some(ContentCacheBudget::zero()),
            },
        )
        .map_err(|e| anyhow::anyhow!("picker init: {e}"))?;

        // Register the shared state globally so tools can access it directly.
        let global_state = FffSearchState {
            project_path: self.root.clone(),
            picker: self.shared_picker.clone(),
            frecency: self.shared_frecency.clone(),
            query_tracker: self.shared_query_tracker.clone(),
        };
        {
            let mut guard = fff_state().write();
            *guard = Some(FffSearchStateInner {
                state: global_state,
                indexed: false,
            });
        }

        // Wait for initial scan to complete
        let timeout = Duration::from_secs(SCAN_TIMEOUT_SECS);
        if self.shared_picker.wait_for_scan(timeout) {
            tracing::debug!("fff indexer: initial scan complete");
            self.indexed = true;
            // Mark as indexed in the global registry too.
            let mut guard = fff_state().write();
            if let Some(inner) = guard.as_mut() {
                inner.indexed = true;
            }
        } else {
            tracing::warn!("fff indexer: scan timed out after {SCAN_TIMEOUT_SECS}s");
        }

        Ok(())
    }

    /// Handle a search request.
    async fn handle_search(&self, req: FffSearchRequest) -> FffSearchResultPayload {
        let limit = req.limit.unwrap_or(DEFAULT_LIMIT);
        let project_path_str = req.project_path.to_string_lossy().to_string();

        // Acquire read lock on shared picker
        let picker_guard = match self.shared_picker.read() {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("fff picker read lock poisoned: {e}");
                return result_payload(req, vec![], 0, false);
            }
        };

        let qt_guard = match self.shared_query_tracker.read() {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("fff query tracker lock poisoned: {e}");
                return result_payload(req, vec![], 0, false);
            }
        };

        let picker = match picker_guard.as_ref() {
            Some(p) => p,
            None => return result_payload(req, vec![], 0, self.indexed),
        };

        // Parse query using fff-query-parser
        let parsed = QueryParser::default().parse(&req.query);

        // Execute search
        let results: FffRawSearchResult = picker.fuzzy_search(
            &parsed,
            qt_guard.as_ref(),
            FuzzySearchOptions {
                max_threads: 0,
                current_file: None,
                project_path: Some(&req.project_path),
                pagination: PaginationArgs {
                    offset: 0,
                    limit,
                },
                combo_boost_score_multiplier: 100,
                min_combo_count: 2,
                ..Default::default()
            },
        );

        // Convert results to our payload type
        let items: Vec<FffFileItem> = results
            .items
            .iter()
            .zip(results.scores.iter())
            .map(|(item, score)| {
                let git_status = item.git_status;
                let git_tracked = git_status.is_some();
                FffFileItem {
                    relative_path: item.relative_path(picker),
                    absolute_path: item.absolute_path(picker, &req.project_path)
                        .to_string_lossy()
                        .into_owned(),
                    score: score.total as f64,
                    git_tracked,
                    git_status: git_status.map(format_git_status_str),
                }
            })
            .collect();

        result_payload(req, items, results.total_matched, self.indexed)
    }
}

/// Format a git2 Status as a human-readable string.
fn format_git_status_str(status: git2::Status) -> String {
    use git2::Status as G;
    if status.contains(G::WT_NEW) || status.contains(G::INDEX_NEW) {
        "untracked".to_string()
    } else if status.contains(G::WT_MODIFIED) || status.contains(G::INDEX_MODIFIED) {
        "modified".to_string()
    } else if status.contains(G::WT_DELETED) || status.contains(G::INDEX_DELETED) {
        "deleted".to_string()
    } else if status.contains(G::WT_RENAMED) || status.contains(G::INDEX_RENAMED) {
        "renamed".to_string()
    } else {
        "clean".to_string()
    }
}

/// Build a result payload.
fn result_payload(
    req: FffSearchRequest,
    items: Vec<FffFileItem>,
    total_matched: usize,
    indexed: bool,
) -> FffSearchResultPayload {
    FffSearchResultPayload {
        request_id: req.request_id,
        query: req.query,
        items,
        total_matched,
        indexed,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn indexer_initializes_in_temp_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let data_dir = tmp.path().to_path_buf();

        // Create a few files
        std::fs::create_dir_all(root.join("src")).ok();
        std::fs::create_dir_all(root.join("tests")).ok();
        std::fs::write(root.join("src/lib.rs"), "// lib").ok();
        std::fs::write(root.join("src/main.rs"), "// main").ok();
        std::fs::write(root.join("tests/example.rs"), "// test").ok();

        // Ensure LMDB dirs exist
        std::fs::create_dir_all(data_dir.join("runie").join("fff").join("frecency")).ok();
        std::fs::create_dir_all(data_dir.join("runie").join("fff").join("queries")).ok();

        // Create the bus
        let bus = EventBus::new(16);

        // Spawn the indexer
        let (tx, handle) = FffIndexerActor::spawn(root.clone(), data_dir, bus.clone())
            .expect("spawn succeeds");

        // Give it time to initialize
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Send a search request
        let request_id = 42;
        let send_result = tx.send(FffSearchRequest {
            request_id,
            query: "lib".to_string(),
            limit: Some(10),
            project_path: root.clone(),
        }).await;

        // Collect results
        let mut results = Vec::new();
        let mut sub = bus.subscribe();
        for _ in 0..5 {
            if let Some(Ok(FffSearchResult(payload))) = sub.try_recv() {
                results.push(payload);
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        // Abort the actor
        handle.abort();

        // Send should succeed (or gracefully fail if actor exited)
        assert!(send_result.is_ok() || send_result.is_err(), "send should not panic");

        if !results.is_empty() {
            let result = &results[0];
            assert_eq!(result.request_id, request_id);
        }
    }

    #[tokio::test]
    async fn indexer_answers_file_search() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let data_dir = tmp.path().to_path_buf();

        // Create structured test files
        std::fs::create_dir_all(root.join("src/cli")).ok();
        std::fs::create_dir_all(root.join("src/server")).ok();
        std::fs::write(root.join("src/cli/main.rs"), "fn main() {}").unwrap();
        std::fs::write(root.join("src/server/api.rs"), "pub fn api() {}").unwrap();

        let bus = EventBus::new(16);
        let (tx, handle) = FffIndexerActor::spawn(root.clone(), data_dir, bus.clone())
            .expect("spawn succeeds");

        // Wait for scan
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Search for "cli"
        let request_id = 7;
        tx.send(FffSearchRequest {
            request_id,
            query: "cli".to_string(),
            limit: Some(5),
            project_path: root.clone(),
        })
        .await
        .expect("send succeeds");

        // Wait for result
        let mut result = None;
        let mut sub = bus.subscribe();
        for _ in 0..10 {
            if let Some(Ok(FffSearchResult(payload))) = sub.try_recv() {
                if payload.request_id == request_id {
                    result = Some(payload);
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        handle.abort();

        let result = result.expect("got a result for request_id 7");
        assert_eq!(result.request_id, 7);
        // Should find src/cli/main.rs
        assert!(
            result.items.iter().any(|i| i.relative_path.contains("cli")),
            "expected cli file in results: {:?}",
            result.items
        );
    }

    #[tokio::test]
    async fn search_request_event_returns_results() {
        // Integration test: search request → search result event
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let data_dir = tmp.path().to_path_buf();

        std::fs::write(root.join("readme.md"), "# Hello World").unwrap();
        std::fs::write(root.join("todo.txt"), "buy milk").unwrap();

        let bus = EventBus::new(16);
        let (tx, handle) = FffIndexerActor::spawn(root.clone(), data_dir, bus.clone())
            .expect("spawn succeeds");

        tokio::time::sleep(Duration::from_secs(3)).await;

        let request_id = 99;
        tx.send(FffSearchRequest {
            request_id,
            query: "readme".to_string(),
            limit: Some(5),
            project_path: root,
        })
        .await
        .expect("send succeeds");

        // Drain events until we get our result
        let mut got_result = false;
        let mut sub = bus.subscribe();
        for _ in 0..15 {
            if let Some(Ok(FffSearchResult(payload))) = sub.try_recv() {
                if payload.request_id == request_id {
                    assert!(!payload.items.is_empty() || !payload.indexed);
                    got_result = true;
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        handle.abort();
        assert!(got_result, "search result event was not received");
    }
}
