//! Core TUI integration tests moved from runie-core.
#![allow(clippy::too_many_lines)] // test files contain many test cases

// Re-export types used by child test modules so they can `use super::*;`
pub use super::{AppState, DialogKind};

#[cfg(test)]
mod action_text;
#[cfg(test)]
mod at_file_picker;
#[cfg(test)]
mod autoscroll_bug;
#[cfg(test)]
mod autoscroll_overflow;
#[cfg(test)]
mod collapse;
#[cfg(test)]
mod collapse_new_items;
#[cfg(test)]
mod dialog_theme_switch;
#[cfg(test)]
mod element_dsl;
#[cfg(test)]
mod element_order;
#[cfg(test)]
mod element_sorting;
#[cfg(test)]
mod element_spacing;
#[cfg(test)]
mod input;
#[cfg(test)]
mod line_scroll;
#[cfg(test)]
mod mouse_events;
#[cfg(test)]
mod no_ghost_agent;
#[cfg(test)]
mod palette;
#[cfg(test)]
mod paragraph_scroll;
#[cfg(test)]
mod scrollbar;
#[cfg(test)]
mod semantic_order;
#[cfg(test)]
mod settings_dialog;
#[cfg(test)]
mod status_timer;
#[cfg(test)]
mod tab_complete;
#[cfg(test)]
mod tab_file_picker_filter;
#[cfg(test)]
mod thinking;
#[cfg(test)]
mod toggle_all;
#[cfg(test)]
mod toggle_stress;
#[cfg(test)]
pub(crate) mod visible_helper;

// =============================================================================
// Shared test helpers
// =============================================================================

/// Injects mock file entries into the open file picker panel.
/// Opens the picker if not already open, then adds 3 test files:
/// `Cargo.toml` (file), `src/` (dir), `README.md` (file).
///
/// Idempotent: safe to call even if picker is already open.
pub fn inject_mock_file_entries(state: &mut AppState) {
    use runie_core::commands::DialogState;
    use runie_core::dialog::{ItemAction, PanelItem};
    use runie_core::Event;

    // Ensure the file picker dialog is open.
    let already_open = matches!(
        state.open_dialog(),
        Some(DialogState::Active { kind: DialogKind::Generic, .. })
    );
    if !already_open {
        runie_core::update::dialog::open_at_file_picker_all(state);
    }

    // Get the panel and push mock items directly.
    if let Some(DialogState::Active { kind: DialogKind::Generic, panels }) = state.open_dialog_mut() {
        if let Some(panel) = panels.current_mut() {
            // header("3 files") is already set by open_at_file_picker_all.
            let mock_entries = vec![
                ("Cargo.toml".to_string(), false, Event::InsertAtRef("Cargo.toml".to_string())),
                ("src/".to_string(), true, Event::InsertAtRef("src/".to_string())),
                ("README.md".to_string(), false, Event::InsertAtRef("README.md".to_string())),
            ];
            for (name, is_dir, evt) in mock_entries {
                let label = if is_dir { name } else { name };
                panel.items.push(PanelItem::Action { label, action: ItemAction::Emit(evt) });
            }
        }
    }
}
