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
            let mut guard = super::fff_state().write();
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
            let mut guard = super::fff_state().write();
            if let Some(inner) = guard.as_mut() {
                inner.indexed = true;
            }
        } else {
            tracing::warn!("fff indexer: scan timed out after {SCAN_TIMEOUT_SECS}s");
        }

        Ok(())
    }

    /// Handle a search request.
    pub(super) async fn handle_search(&self, req: FffSearchRequest) -> FffSearchResultPayload {
        let limit = req.limit.unwrap_or(DEFAULT_LIMIT);
        let _project_path_str = req.project_path.to_string_lossy().to_string();

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
                pagination: PaginationArgs { offset: 0, limit },
                combo_boost_score_multiplier: 100,
                min_combo_count: 2,
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
                    absolute_path: item
                        .absolute_path(picker, &req.project_path)
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
