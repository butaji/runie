//! Panel activation and action handling.

use super::settings::apply_panel_setting;
use crate::dialog::{ItemAction, PanelItem, PanelStack};
use crate::model::{AppState, InputReceiver};
use crate::Event;

/// Result of handling panel activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActivationResult {
    /// Event was consumed and the dialog should remain open.
    Consumed,
    /// Event closed the dialog.
    Closed,
}

/// Handle panel activation events (Enter/Select).
pub fn handle_panel_activation(
    state: &mut AppState,
    event: &Event,
    stack: &mut PanelStack,
) -> Option<ActivationResult> {
    match event {
        Event::Submit
        | Event::SettingsSelect
        | Event::PaletteSelect
        | Event::ModelSelectorSelect => {
            return Some(try_activate_panel(state, stack));
        }
        Event::Input(' ') => {
            if let Some(panel) = stack.current_mut() {
                if super::toggle_selected_checkbox(state, panel) {
                    return Some(ActivationResult::Consumed);
                }
            }
        }
        // intentionally ignored: other events fall through
        _ => {}
    }
    None
}

/// Try to activate the currently selected panel item.
pub fn try_activate_panel(state: &mut AppState, stack: &mut PanelStack) -> ActivationResult {
    if let Some(action) = stack.activate() {
        if handle_panel_action(state, action, stack) {
            return ActivationResult::Closed;
        }
    }
    ActivationResult::Consumed
}

/// Handle a panel item action. Returns `true` if the dialog was closed.
pub fn handle_panel_action(
    state: &mut AppState,
    action: ItemAction,
    stack: &mut PanelStack,
) -> bool {
    match action {
        ItemAction::Push(_) | ItemAction::Pop => {
            stack.pop();
            false
        }
        ItemAction::Close => {
            *state.open_dialog_mut() = None;
            state.view_mut().input_receiver = InputReceiver::ChatInput;
            state.view_mut().dirty = true;
            true
        }
        ItemAction::Emit(evt) => handle_emit_action(state, stack, evt),
        ItemAction::Toggle(key) => {
            panel_toggle_item(state, stack, &key);
            // Toggling a checkbox (Enter or Space) keeps the dialog open so the
            // user can flip several settings in a row; only Esc / an explicit
            // Close action dismisses it. Previously Enter toggled and then
            // closed the dialog, which looked like Enter merely dismissed it.
            false
        }
        ItemAction::Cycle(key) => {
            panel_cycle_item(state, stack, &key);
            // Cycling a settings select (Enter) keeps the dialog open so the
            // user can adjust several rows in a row; only Esc / an explicit
            // Close action dismisses it. Matches the Toggle arm behaviour.
            false
        }
    }
}

/// Handle emit actions (running commands from palette).
pub fn handle_emit_action(state: &mut AppState, stack: &mut PanelStack, evt: Event) -> bool {
    let keep_open = stack
        .current()
        .map(|p| p.keep_open_on_activate)
        .unwrap_or(false);
    if !keep_open {
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = InputReceiver::ChatInput;
    }
    state.view_mut().dirty = true;

    // For RunPaletteCommand, pass the panel filter as args
    let evt = if let Event::RunPaletteCommand { name, args } = &evt {
        if args.is_empty() {
            if let Some(panel) = stack.current() {
                let filter_args = extract_palette_args(name, &panel.filter);
                Event::RunPaletteCommand {
                    name: name.clone(),
                    args: filter_args,
                }
            } else {
                evt.clone()
            }
        } else {
            evt.clone()
        }
    } else {
        evt
    };

    state.update(evt);
    !keep_open
}

/// Extract args from panel filter for RunPaletteCommand.
///
/// Only an exact `"<name> <args>"` filter — the user typing the full command
/// line into the palette — yields args. A fuzzy search fragment that merely
/// matches the command (e.g. "the" matching "theme") is a search query,
/// never an argument.
pub fn extract_palette_args(name: &str, filter: &str) -> String {
    let filter = filter.trim();
    match filter.strip_prefix(name) {
        Some(rest) if rest.is_empty() || rest.starts_with(' ') => rest.trim().to_owned(),
        _ => String::new(),
    }
}

/// Toggle the currently selected item.
pub fn panel_toggle_item(state: &mut AppState, stack: &mut PanelStack, _key: &str) {
    if let Some(panel) = stack.current_mut() {
        let _ = super::toggle_selected_checkbox(state, panel);
    }
}

/// Cycle through options for the currently selected item.
pub fn panel_cycle_item(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    if let Some(PanelItem::Select {
        current, options, ..
    }) = stack.current_mut().and_then(|p| p.selected_item_mut())
    {
        if let Some(idx) = options.iter().position(|o| o == current) {
            let next = (idx + 1) % options.len();
            *current = options[next].clone();
        }
    }
    apply_panel_setting(state, stack, key);
}
