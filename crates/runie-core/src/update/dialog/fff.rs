//! FFF file search query and git status formatting.

use crate::model::FffFileEntry;
use crate::FffSearchState;

/// Query the global FFF index for fuzzy file search results.
/// Returns up to `limit` entries ranked by frecency + fuzzy score.
/// Query FFF indexer for files. Falls back to file_refs if FFF is unavailable or
/// returns no results (e.g. not indexed yet in tests).
pub(crate) fn query_fff_files(query: &str, limit: usize) -> Vec<FffFileEntry> {
    let fallback = build_fff_fallback(limit);
    let Some(state) = FffSearchState::get() else {
        return fallback;
    };
    let Ok(picker_guard) = state.picker.read() else {
        return fallback;
    };
    let Ok(qt_guard) = state.query_tracker.read() else {
        return fallback;
    };
    let Some(picker) = picker_guard.as_ref() else {
        return fallback;
    };
    let query_tracker = qt_guard.as_ref();

    let parsed = fff_search::QueryParser::default().parse(query);
    let results = picker.fuzzy_search(
        &parsed,
        query_tracker,
        fff_search::FuzzySearchOptions {
            max_threads: 0,
            current_file: None,
            project_path: None,
            pagination: fff_search::PaginationArgs { offset: 0, limit },
            combo_boost_score_multiplier: 100,
            min_combo_count: 2,
        },
    );
    if results.items.is_empty() {
        return fallback;
    }
    format_fff_results(picker, &results)
}

fn build_fff_fallback(limit: usize) -> Vec<FffFileEntry> {
    crate::file_refs::find_file_entries(".", limit)
        .into_iter()
        .map(|e| FffFileEntry {
            name: e.name.clone(),
            path: e.name,
            is_dir: e.is_dir,
            score: 0.0,
            git_status: None,
        })
        .collect()
}

fn format_fff_results(
    picker: &fff_search::FilePicker,
    results: &fff_search::SearchResult,
) -> Vec<FffFileEntry> {
    results
        .items
        .iter()
        .zip(results.scores.iter())
        .map(|(item, score)| {
            let path = item.relative_path(picker);
            let name = std::path::Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            let is_dir = path.ends_with('/') || item.relative_path(picker).is_empty();
            let git_status = item.git_status.map(format_fff_git_status);
            FffFileEntry {
                name,
                path,
                is_dir,
                score: score.total as f64,
                git_status,
            }
        })
        .collect()
}

fn format_fff_git_status(status: git2::Status) -> String {
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
    String::new()
}
