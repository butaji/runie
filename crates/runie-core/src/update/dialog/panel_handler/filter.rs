//! Panel filtering handling.

use crate::dialog::PanelStack;
use crate::model::AppState;
use crate::Event;

/// Handle panel filter events (typing in the input box).
pub fn handle_panel_filter(_state: &mut AppState, event: &Event, stack: &mut PanelStack) {
    let accepts_text = stack
        .current()
        .is_some_and(|p| !p.vim_picker_enabled || p.vim_filter_mode);
    match event {
        Event::PaletteFilter(c) if accepts_text => stack.push_filter(*c),
        Event::ModelSelectorFilter(c) if accepts_text => stack.push_filter(*c),
        Event::Input('i')
            if stack
                .current()
                .is_some_and(|p| p.vim_picker_enabled && !p.vim_filter_mode) =>
        {
            if let Some(panel) = stack.current_mut() {
                panel.set_vim_filter_mode(true);
            }
        }
        Event::Input(c) if accepts_text => {
            stack.push_filter(*c);
        }
        Event::PaletteBackspace | Event::ModelSelectorBackspace | Event::Backspace if accepts_text => {
            stack.pop_filter();
        }
        // intentionally ignored: other filter events are no-ops
        _ => {}
    }
}
