//! File picker helpers and FFF search query (merged from fff.rs and file_picker.rs).

use crate::commands::DialogState;
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::{AppState, FffFileEntry};

// ---------------------------------------------------------------------------
// FFF file search
// ---------------------------------------------------------------------------

/// Query the global FFF index for fuzzy file search results.
/// Returns up to `limit` entries ranked by frecency + fuzzy score.
pub(crate) fn query_fff_files(query: &str, limit: usize) -> Vec<FffFileEntry> {
    let fallback = build_fff_fallback(limit);
    let Some(state) = crate::FffSearchState::get() else {
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
    crate::async_io::block_in_place_if_runtime(|| crate::file_refs::find_file_entries(".", limit))
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

/// Format a git2 Status for FFF file picker results.
/// Returns empty string for clean state (no tracked changes).
fn format_fff_git_status(status: git2::Status) -> String {
    let label = crate::actors::format_git_status(status);
    if label == "clean" {
        String::new()
    } else {
        label.to_owned()
    }
}

// ---------------------------------------------------------------------------
// File picker panel building and rebuilding
// ---------------------------------------------------------------------------

pub(crate) fn build_file_picker_panel(
    mut panel: Panel,
    entries: &[FffFileEntry],
    filter: Option<&str>,
) -> Panel {
    let header = file_picker_header(entries.len(), filter);
    panel = panel.header(&header);
    for entry in entries {
        let label = file_picker_label(entry);
        let insert_name = file_picker_insert_name(entry);
        panel = panel.item(
            &label,
            ItemAction::Emit(crate::Event::InsertAtRef(insert_name)),
        );
    }
    panel
}

fn file_picker_header(count: usize, filter: Option<&str>) -> String {
    if filter.map(|f| !f.is_empty()).unwrap_or(false) {
        format!("{} files matching '{}'", count, filter.unwrap_or(""))
    } else {
        format!("{} files", count)
    }
}

fn file_picker_label(entry: &FffFileEntry) -> String {
    let name = if entry.is_dir {
        format!("{}/", entry.name)
    } else {
        entry.name.clone()
    };
    if let Some(status) = &entry.git_status {
        if !status.is_empty() {
            return format!("{} {}", status, name);
        }
    }
    name
}

fn file_picker_insert_name(entry: &FffFileEntry) -> String {
    if entry.is_dir {
        format!("{}/", entry.path)
    } else {
        entry.path.clone()
    }
}

/// Rebuild the file picker panel with the current FFF results and panel filter.
pub(crate) fn rebuild_file_picker(state: &mut AppState) {
    let Some(DialogState::PanelStack(stack)) = state.open_dialog() else {
        return;
    };
    let Some(panel) = stack.current() else {
        return;
    };
    let filter = panel.filter.clone();
    let query = if filter.is_empty() { "" } else { &filter };
    refresh_file_picker_search(state, query);
    let entries = state.fff_file_results();
    let new_panel = build_picker_panel_with_results(&filter, entries);
    *state.open_dialog_mut() = Some(DialogState::PanelStack(PanelStack::new(new_panel)));
    state.view_mut().dirty = true;
}

/// Refresh file picker results — try actor first, fall back to sync query.
fn refresh_file_picker_search(state: &mut AppState, query: &str) {
    if let Some(ref handles) = state.actor_handles() {
        if let Some(ref fff) = handles.fff_indexer {
            let request_id = state.fff_debounce().wrapping_add(1);
            let request = crate::actors::FffSearchRequest {
                request_id,
                query: query.to_owned(),
                limit: Some(50),
                project_path: std::env::current_dir().unwrap_or_default(),
            };
            fff.try_search(request);
            *state.fff_debounce_mut() = request_id;
            return;
        }
    }
    // Fall back to synchronous query.
    let entries = query_fff_files(query, 50);
    *state.fff_file_results_mut() = entries;
}

fn build_picker_panel_with_results(filter: &str, entries: &[FffFileEntry]) -> Panel {
    let mut new_panel = Panel::new("at-files", " Files ").with_filter();
    new_panel.filter = filter.to_owned();

    let count = entries.len();
    if entries.is_empty() {
        return new_panel.header("No files found");
    }

    let header = file_picker_header(count, Some(filter));
    new_panel = new_panel.header(&header);
    for entry in entries {
        let label = file_picker_label(entry);
        let insert_name = file_picker_insert_name(entry);
        new_panel = new_panel.item(
            &label,
            ItemAction::Emit(crate::Event::InsertAtRef(insert_name)),
        );
    }
    new_panel
}
