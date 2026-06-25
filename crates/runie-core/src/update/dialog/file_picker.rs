//! File picker panel building and rebuilding.

use crate::commands::DialogState;
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::{AppState, FffFileEntry};

use super::fff::query_fff_files;

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
    if entry.is_dir {
        format!("{}/", entry.name)
    } else {
        entry.name.clone()
    }
}

fn file_picker_insert_name(entry: &FffFileEntry) -> String {
    // Use full relative path so frecency can record access by path.
    if entry.is_dir {
        format!("{}/", entry.path)
    } else {
        entry.path.clone()
    }
}

/// Rebuild the file picker panel with the current FFF results and panel filter.
/// Called when the user types in the file picker to update fuzzy results.
pub(crate) fn rebuild_file_picker(state: &mut AppState) {
    let Some(DialogState::PanelStack(stack)) = state.open_dialog() else {
        return;
    };
    let Some(panel) = stack.current() else {
        return;
    };
    let filter = panel.filter.clone();

    // Re-query FFF with the new filter.
    let query = if filter.is_empty() { "" } else { &filter };
    let entries = query_fff_files(query, 50);
    state.fff_file_results = entries.clone();
    *state.fff_debounce_mut() = state.fff_debounce().wrapping_add(1);

    let mut new_panel = Panel::new("at-files", " Files ").with_filter();
    new_panel.filter = filter.clone();

    let count = entries.len();
    new_panel = if entries.is_empty() {
        new_panel.header("No files found")
    } else {
        let header = file_picker_header(count, Some(&filter));
        new_panel = new_panel.header(&header);
        for entry in entries {
            let label = file_picker_label(&entry);
            let insert_name = file_picker_insert_name(&entry);
            new_panel = new_panel.item(
                &label,
                ItemAction::Emit(crate::Event::InsertAtRef(insert_name)),
            );
        }
        new_panel
    };

    *state.open_dialog_mut() = Some(DialogState::PanelStack(PanelStack::new(new_panel)));
    state.view_mut().dirty = true;
}
