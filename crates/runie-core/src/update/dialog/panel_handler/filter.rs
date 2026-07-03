//! Panel filtering handling.

use super::super::rebuild_file_picker;
use crate::dialog::PanelStack;
use crate::model::AppState;
use crate::Event;

/// Handle panel filter events (typing in the input box).
pub fn handle_panel_filter(state: &mut AppState, event: &Event, stack: &mut PanelStack) {
    match event {
        Event::PaletteFilter(c) => stack.push_filter(*c),
        Event::ModelSelectorFilter(c) => stack.push_filter(*c),
        Event::Input(c) => {
            let is_file_picker = stack.current().is_some_and(|p| p.id == "at-files");
            stack.push_filter(*c);
            // If this is the file picker, re-query FFF with the new filter.
            // Read `is_file_picker` BEFORE calling rebuild_file_picker (which replaces open_dialog).
            if is_file_picker {
                rebuild_file_picker(state);
            }
        }
        Event::PaletteBackspace | Event::ModelSelectorBackspace | Event::Backspace => {
            let is_file_picker = stack.current().is_some_and(|p| p.id == "at-files");
            stack.pop_filter();
            if is_file_picker {
                rebuild_file_picker(state);
            }
        }
        // intentionally ignored: other filter events are no-ops
        _ => {}
    }
}
