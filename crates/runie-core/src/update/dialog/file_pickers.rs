//! File picker helpers — panel building and rebuilding.
//! FFF search results come exclusively via `Event::FffSearchResult` from `FffIndexerActor`.

use crate::commands::{DialogKind, DialogState};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::{AppState, FffFileEntry};

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
/// Results are populated asynchronously via `Event::FffSearchResult`.
pub(crate) fn rebuild_file_picker(state: &mut AppState) {
    let Some(DialogState::Active { kind: DialogKind::Generic, panels: stack }) = state.open_dialog() else {
        return;
    };
    let Some(panel) = stack.current() else {
        return;
    };
    let filter = panel.filter.clone();
    let query = if filter.is_empty() { "" } else { &filter };

    // Send search request to actor; results arrive via Event::FffSearchResult.
    super::open::refresh_file_picker_search(state, query);

    // Use current results (may be stale until actor responds).
    let entries = state.fff_file_results();
    let new_panel = build_picker_panel_with_results(&filter, entries);
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::Generic, panels: PanelStack::new(new_panel) });
    state.view_mut().dirty = true;
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
