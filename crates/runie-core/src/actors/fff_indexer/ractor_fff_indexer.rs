//! Ractor-based `FffIndexerActor` implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::path::PathBuf;

use ractor::{Actor, ActorProcessingErr, ActorRef, async_trait};
use fff_search::{
    ContentCacheBudget, FFFMode, FilePickerOptions, FrecencyTracker, FuzzySearchOptions,
    PaginationArgs, QueryParser, QueryTracker, SearchResult as FffRawSearchResult,
};
use std::time::Duration;

use crate::actors::ractor_adapter::{spawn_ractor, RactorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::FffFileEntry;

use super::{
    FffFileItem, FffSearchRequest, FffSearchResultPayload, FffSearchState,
    FffSearchStateInner, DEFAULT_LIMIT, SCAN_TIMEOUT_SECS,
};

/// Ractor-based FffIndexerActor handle.
#[derive(Clone, Debug)]
pub struct RactorFffIndexerHandle {
    inner: RactorHandle<FffSearchRequest>,
}

impl RactorFffIndexerHandle {
    /// Create a new handle wrapping the inner RactorHandle.
    pub fn new(inner: RactorHandle<FffSearchRequest>) -> Self {
        Self { inner }
    }

    /// Send a search request to the indexer.
    pub async fn search(&self, request: FffSearchRequest) {
        self.inner.send(request).await;
    }

    /// Try to send a search request (non-blocking).
    pub fn try_search(&self, request: FffSearchRequest) {
        let _ = self.inner.try_send(request);
    }
}

/// Ractor-based FffIndexerActor.
pub struct RactorFffIndexerActor {
    root: PathBuf,
    frecency_path: PathBuf,
    query_path: PathBuf,
    bus: EventBus<Event>,
    shared_picker: fff_search::SharedFilePicker,
    shared_frecency: fff_search::SharedFrecency,
    shared_query_tracker: fff_search::SharedQueryTracker,
    indexed: bool,
    init_done: bool,
}

impl RactorFffIndexerActor {
    fn new(root: PathBuf, data_dir: PathBuf, bus: EventBus<Event>) -> Self {
        let fff_dir = data_dir.join("runie").join("fff");
        Self {
            root,
            frecency_path: fff_dir.join("frecency"),
            query_path: fff_dir.join("queries"),
            bus, 
            shared_picker: fff_search::SharedFilePicker::default(),
            shared_frecency: fff_search::SharedFrecency::default(),
            shared_query_tracker: fff_search::SharedQueryTracker::default(),
            indexed: false,
            init_done: false,
        }
    }

    /// Spawn a `RactorFffIndexerActor` and return a handle + cell.
    /// Initializes the FFF stores and waits for initial scan before returning.
    pub async fn spawn(
        root: PathBuf,
        data_dir: PathBuf,
        bus: EventBus<Event>,
    ) -> Result<(RactorFffIndexerHandle, ractor::ActorCell), ractor::SpawnErr> {
        let mut actor = Self::new(root, data_dir, bus.clone());
        // Initialize FFF stores synchronously before spawning
        if let Err(e) = actor.init_fff().await {
            tracing::error!("fff indexer init failed: {e}");
        }
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await?;
        Ok((RactorFffIndexerHandle::new(handle), cell))
    }
}

#[async_trait]
impl Actor for RactorFffIndexerActor {
    type Msg = FffSearchRequest;
    type State = ();
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let payload = self.handle_search(msg).await;
        let entries = payload
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
        self.emit(Event::FffSearchResult {
            request_id: payload.request_id,
            entries,
            query: payload.query,
            indexed: payload.indexed,
        });
        Ok(())
    }
}

impl RactorFffIndexerActor {
    /// Initialize FFF: open LMDB stores, start the picker, and register globally.
    async fn init_fff(&mut self) -> anyhow::Result<()> {
        self.open_fff_stores()?;
        self.create_fff_picker()?;
        self.register_fff_state();
        self.wait_for_fff_scan().await;
        self.init_done = true;
        Ok(())
    }

    fn open_fff_stores(&mut self) -> anyhow::Result<()> {
        let frecency = FrecencyTracker::open(&self.frecency_path)
            .map_err(|e| anyhow::anyhow!("frecency open: {e}"))?;
        self.shared_frecency
            .init(frecency)
            .map_err(|e| anyhow::anyhow!("frecency init: {e}"))?;

        let query_tracker = QueryTracker::open(&self.query_path)
            .map_err(|e| anyhow::anyhow!("query tracker open: {e}"))?;
        self.shared_query_tracker
            .init(query_tracker)
            .map_err(|e| anyhow::anyhow!("query tracker init: {e}"))?;
        Ok(())
    }

    fn create_fff_picker(&self) -> anyhow::Result<()> {
        let root_str = self.root.to_string_lossy().into_owned();
        fff_search::FilePicker::new_with_shared_state(
            self.shared_picker.clone(),
            self.shared_frecency.clone(),
            FilePickerOptions {
                base_path: root_str,
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
        Ok(())
    }

    fn register_fff_state(&self) {
        let global_state = FffSearchState {
            project_path: self.root.clone(),
            picker: self.shared_picker.clone(),
            frecency: self.shared_frecency.clone(),
            query_tracker: self.shared_query_tracker.clone(),
        };
        *super::fff_state().write() = Some(FffSearchStateInner {
            state: global_state,
            indexed: false,
        });
    }

    async fn wait_for_fff_scan(&mut self) {
        let timeout = Duration::from_secs(SCAN_TIMEOUT_SECS);
        let picker = self.shared_picker.clone();
        let completed =
            match tokio::task::spawn_blocking(move || picker.wait_for_scan(timeout)).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("fff indexer: scan task failed: {e}");
                    return;
                }
            };
        if !completed {
            tracing::warn!("fff indexer: scan timed out after {SCAN_TIMEOUT_SECS}s");
            return;
        }
        tracing::debug!("fff indexer: initial scan complete");
        self.indexed = true;
        let mut guard = super::fff_state().write();
        if let Some(inner) = guard.as_mut() {
            inner.indexed = true;
        }
    }

    /// Handle a search request.
    async fn handle_search(&self, req: FffSearchRequest) -> FffSearchResultPayload {
        let limit = req.limit.unwrap_or(DEFAULT_LIMIT);

        let picker_guard = match self.shared_picker.read() {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("fff picker read lock poisoned: {e}");
                return result_payload(&req, vec![], 0, false);
            }
        };
        let qt_guard = match self.shared_query_tracker.read() {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("fff query tracker lock poisoned: {e}");
                return result_payload(&req, vec![], 0, false);
            }
        };
        let picker = match picker_guard.as_ref() {
            Some(p) => p,
            None => return result_payload(&req, vec![], 0, self.indexed),
        };
        let query_tracker = qt_guard.as_ref();

        let results = run_fff_search(&req, picker, query_tracker, limit);
        let items = convert_fff_results(&req, picker, &results);
        result_payload(&req, items, results.total_matched, self.indexed)
    }

    fn emit(&self, event: Event) {
        self.bus.publish(event);
    }
}

/// Build a result payload.
fn run_fff_search<'a>(
    req: &'a FffSearchRequest,
    picker: &'a fff_search::FilePicker,
    query_tracker: Option<&'a QueryTracker>,
    limit: usize,
) -> FffRawSearchResult<'a> {
    let parsed = QueryParser::default().parse(&req.query);
    picker.fuzzy_search(
        &parsed,
        query_tracker,
        FuzzySearchOptions {
            max_threads: 0,
            current_file: None,
            project_path: Some(&req.project_path),
            pagination: PaginationArgs { offset: 0, limit },
            combo_boost_score_multiplier: 100,
            min_combo_count: 2,
        },
    )
}

fn convert_fff_results(
    req: &FffSearchRequest,
    picker: &fff_search::FilePicker,
    results: &FffRawSearchResult,
) -> Vec<FffFileItem> {
    results
        .items
        .iter()
        .zip(results.scores.iter())
        .map(|(item, score)| {
            let git_status = item.git_status;
            let git_tracked = git_status.is_some();
            FffFileItem {
                relative_path: item.relative_path(picker),
                absolute_path: item
                    .absolute_path(picker, &req.project_path)
                    .to_string_lossy()
                    .into_owned(),
                score: score.total as f64,
                git_tracked,
                git_status: git_status.map(format_git_status_str),
            }
        })
        .collect()
}

fn result_payload(
    req: &FffSearchRequest,
    items: Vec<FffFileItem>,
    total_matched: usize,
    indexed: bool,
) -> FffSearchResultPayload {
    FffSearchResultPayload {
        request_id: req.request_id,
        query: req.query.clone(),
        items,
        total_matched,
        indexed,
    }
}

/// Map a git2 Status to a human-readable label.
///
/// Returns `"clean"` if no tracked status flags are set.
pub fn format_git_status(status: git2::Status) -> &'static str {
    use git2::Status as G;
    const STATUS_LABELS: &[(git2::Status, &str)] = &[
        (G::WT_NEW, "untracked"),
        (G::INDEX_NEW, "untracked"),
        (G::WT_MODIFIED, "modified"),
        (G::INDEX_MODIFIED, "modified"),
        (G::WT_DELETED, "deleted"),
        (G::INDEX_DELETED, "deleted"),
        (G::WT_RENAMED, "renamed"),
        (G::INDEX_RENAMED, "renamed"),
    ];
    for (flag, label) in STATUS_LABELS {
        if status.contains(*flag) {
            return label;
        }
    }
    "clean"
}

/// Format a git2 Status as a human-readable string (returns `"clean"` for clean state).
fn format_git_status_str(status: git2::Status) -> String {
    format_git_status(status).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ractor_fff_indexer_actor_spawns() {
        use crate::bus::EventBus;
        use crate::event::Event;
        
        FffSearchState::reset_for_test();
        let bus = EventBus::<Event>::new(16);
        let temp_dir = tempfile::tempdir().unwrap();
        let result = RactorFffIndexerActor::spawn(
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
            bus,
        ).await;
        assert!(result.is_ok());
    }
}
