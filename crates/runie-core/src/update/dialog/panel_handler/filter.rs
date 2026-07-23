//! Panel filtering handling.

use crate::dialog::PanelStack;
use crate::model::AppState;
use crate::Event;

/// Handle panel filter events (typing in the input box).
pub fn handle_panel_filter(state: &mut AppState, event: &Event, stack: &mut PanelStack) {
    match event {
        Event::PaletteFilter(c) => stack.push_filter(*c),
        Event::ModelSelectorFilter(c) => stack.push_filter(*c),
        Event::Input(c) => {
            stack.push_filter(*c);
        }
        Event::PaletteBackspace | Event::ModelSelectorBackspace | Event::Backspace => {
            stack.pop_filter();
        }
        // intentionally ignored: other filter events are no-ops
        _ => {}
    }
}
