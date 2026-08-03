//! Panel navigation and close handling.

use crate::dialog::PanelStack;
use crate::model::AppState;
use crate::Event;

/// Grok picker contract: searchable dialogs open in input mode; Esc first
/// clears a query, then switches to navigation mode before a later Esc closes.
pub fn handle_vim_picker_escape(state: &mut AppState, event: &Event, stack: &mut PanelStack) -> bool {
    if !matches!(event, Event::DialogBack) {
        return false;
    }
    let Some(panel) = stack.current_mut() else { return false };
    // Slash autocomplete is an ephemeral composer route: Esc should close it
    // immediately and return to the input. Ctrl+P/opened command palettes use
    // the Grok Vim picker state machine.
    if !state.config().vim_mode || !panel.vim_picker_enabled || state.command_palette_from_input {
        return false;
    }
    if !panel.filter.is_empty() {
        panel.set_filter("");
        return true;
    }
    if panel.vim_filter_mode {
        panel.set_vim_filter_mode(false);
        return true;
    }
    false
}

/// Handle panel close events (Escape, close button, back).
pub fn handle_panel_close(state: &mut AppState, event: &Event, stack: &mut PanelStack) -> bool {
    match event {
        Event::SettingsClose | Event::PaletteClose | Event::ModelSelectorClose | Event::DialogBack => {
            if stack.len() > 1 {
                stack.pop();
            } else {
                let root_closable = stack.root().map(|p| p.closable).unwrap_or(true);
                return pop_dialog_or_close(state, root_closable);
            }
        }
        // intentionally ignored: PanelPop events are handled above in the specific arm
        _ => {}
    }
    false
}

/// Handle panel navigation events (up/down/left/right).
pub fn handle_panel_navigation(_state: &mut AppState, event: &Event, stack: &mut PanelStack) -> bool {
    if let Some(panel) = stack.current_mut() {
        if matches!(event, Event::CursorRight) && panel.toggle_inline_help(true) {
            return true;
        }
        if matches!(event, Event::CursorLeft) && panel.toggle_inline_help(false) {
            return true;
        }
    }
    match event {
        Event::HistoryPrev | Event::SettingsUp | Event::PaletteUp | Event::ModelSelectorUp => {
            stack.select_up();
            return true;
        }
        Event::HistoryNext | Event::SettingsDown | Event::PaletteDown | Event::ModelSelectorDown => {
            stack.select_down();
            return true;
        }
        Event::CursorLeft | Event::SettingsLeft => {
            stack.pop();
            return true;
        }
        Event::Input('\t') => {
            stack.select_down();
            return true;
        }
        Event::CycleThinkingLevel => {
            stack.select_up();
            return true;
        }
        // intentionally ignored: other input events fall through
        _ => {}
    }
    false
}

/// Pop the current dialog or close it entirely.
pub fn pop_dialog_or_close(state: &mut AppState, root_closable: bool) -> bool {
    if !root_closable {
        // The root panel has asked to stay open.
        state.view_mut().dirty = true;
        return false;
    }
    if let Some(previous) = state.dialog_back_stack_mut().pop() {
        *state.open_dialog_mut() = Some(previous);
        state.view_mut().dirty = true;
        false
    } else {
        *state.open_dialog_mut() = None;
        // NOTE: Do NOT reset input_receiver here. handle_vim_dialog_back()
        // checks input_receiver == Dialog to know a dialog was closed and
        // should NOT trigger vim-nav. It will reset input_receiver itself.
        state.view_mut().dirty = true;
        true
    }
}
