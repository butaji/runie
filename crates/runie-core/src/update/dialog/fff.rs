//! FFF file search query and git status formatting.

use crate::model::FffFileEntry;
use crate::FffSearchState;

/// Query the global FFF index for fuzzy file search results.
/// Returns up to `limit` entries ranked by frecency + fuzzy score.
/// Query FFF indexer for files. Falls back to file_refs if FFF is unavailable or
/// returns no results (e.g. not indexed yet in tests).
pub(crate) fn query_fff_files(query: &str, limit: usize) -> Vec<FffFileEntry> {
    // Primary: use file_refs (deterministic, sorted by name).
    let fallback: Vec<FffFileEntry> = crate::file_refs::find_file_entries(".", limit)
        .into_iter()
        .map(|e| FffFileEntry {
            name: e.name.clone(),
            path: e.name,
            is_dir: e.is_dir,
            score: 0.0,
            git_status: None,
        })
        .collect();

    let Some(state) = FffSearchState::get() else {
        return fallback;
    };

    let picker_guard = match state.picker.read() {
        Ok(g) => g,
        Err(_) => return fallback,
    };
    let qt_guard = match state.query_tracker.read() {
        Ok(g) => g,
        Err(_) => return fallback,
    };

    let picker = match picker_guard.as_ref() {
        Some(p) => p,
        None => return fallback,
    };

    let parsed = fff_search::QueryParser::default().parse(query);
    let results = picker.fuzzy_search(
        &parsed,
        qt_guard.as_ref(),
        fff_search::FuzzySearchOptions {
            max_threads: 0,
            current_file: None,
            project_path: None,
            pagination: fff_search::PaginationArgs { offset: 0, limit },
            combo_boost_score_multiplier: 100,
            min_combo_count: 2,
        },
    );

    // If FFF returned no results (not indexed yet), use file_refs fallback.
    if results.items.is_empty() {
        return fallback;
    }

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
    if status.contains(G::WT_NEW) || status.contains(G::INDEX_NEW) {
        "untracked".to_string()
    } else if status.contains(G::WT_MODIFIED) || status.contains(G::INDEX_MODIFIED) {
        "modified".to_string()
    } else if status.contains(G::WT_DELETED) || status.contains(G::INDEX_DELETED) {
        "deleted".to_string()
    } else if status.contains(G::WT_RENAMED) || status.contains(G::INDEX_RENAMED) {
        "renamed".to_string()
    } else {
        String::new()
    }
}
