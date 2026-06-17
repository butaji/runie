use fff_search::{
    ContentCacheBudget, FFFMode, FilePickerOptions, FrecencyTracker, FuzzySearchOptions,
    PaginationArgs, QueryParser, QueryTracker, SearchResult as FffRawSearchResult,
};
use git2;
use std::time::Duration;

use super::{
    FffFileItem, FffIndexerActor, FffSearchRequest, FffSearchResultPayload, FffSearchState,
    FffSearchStateInner, DEFAULT_LIMIT, SCAN_TIMEOUT_SECS,
};

impl FffIndexerActor {
    /// Initialize FFF: open LMDB stores, start the picker, and register globally.
    pub(super) async fn init_fff(&mut self) -> anyhow::Result<()> {
        self.open_fff_stores()?;
        self.create_fff_picker()?;
        self.register_fff_state();
        self.wait_for_fff_scan().await;
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
        Ok(())
    }

    fn register_fff_state(&self) {
        let global_state = FffSearchState {
            project_path: self.root.clone(),
            picker: self.shared_picker.clone(),
            frecency: self.shared_frecency.clone(),
            query_tracker: self.shared_query_tracker.clone(),
        };
        let mut guard = super::fff_state().write();
        *guard = Some(FffSearchStateInner {
            state: global_state,
            indexed: false,
        });
    }

    async fn wait_for_fff_scan(&mut self) {
        let timeout = Duration::from_secs(SCAN_TIMEOUT_SECS);
        let picker = self.shared_picker.clone();
        let completed = match tokio::task::spawn_blocking(move || picker.wait_for_scan(timeout))
            .await
        {
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
    pub(super) async fn handle_search(&self, req: FffSearchRequest) -> FffSearchResultPayload {
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
}

/// Format a git2 Status as a human-readable string.
fn format_git_status_str(status: git2::Status) -> String {
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
            return (*label).to_string();
        }
    }
    "clean".to_string()
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
