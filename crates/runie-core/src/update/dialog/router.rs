//! Dialog routing, command-result processing, and back-stack helpers.
//!
//! ## Borrow pattern
//! Dialog updates require `open_dialog.take()` to temporarily move the dialog out of
//! `AppState`. This is a legitimate borrow-conflict workaround: `&mut AppState` cannot
//! be held while also holding `&mut DialogState` (same struct). The take/process/restore
//! pattern is unavoidable without restructuring AppState's dialog ownership.

use crate::commands::{CommandResult, DialogKind, DialogState, DialogType};
use crate::model::AppState;
use crate::Event;

use super::{
    open_command_palette, open_model_selector, open_scoped_models_dialog, open_settings_dialog,
    open_theme_selector,
    panel_handler::{update_panel_stack, PanelUpdateResult},
};

/// Handles dialog-specific events. Returns whether the dialog was closed.
pub fn update_dialog(state: &mut AppState, event: Event) {
    if route_global_dialog_event(state, &event) {
        return;
    }
    let Some(mut dialog) = state.open_dialog_mut().take() else {
        return;
    };
    // Welcome dialog has no panel stack — only close on specific events
    if matches!(dialog, crate::commands::DialogState::Welcome) {
        *state.open_dialog_mut() = Some(dialog);
        return;
    }
    let is_dialog_back = matches!(&event, Event::DialogBack);
    let is_palette_activation = is_palette_activation(&dialog, &event);
    if is_palette_activation {
        state.push_dialog_to_back_stack(dialog.clone());
    }
    let stack = dialog
        .panel_stack_mut()
        .expect("non-welcome dialog has panel stack");
    let result = update_panel_stack(state, event, stack);
    restore_or_pop_dialog(state, dialog, result, is_palette_activation);

    if is_dialog_back && state.open_dialog().is_none() {
        state.handle_vim_dialog_back();
    }

    state.view_mut().dirty = true;
}

fn route_global_dialog_event(state: &mut AppState, event: &Event) -> bool {
    if matches!(event, crate::Event::Abort) {
        if let Some((input, _, _, _)) = state.input_mut().file_picker_backup.take() {
            state.input_mut().input = input;
            state.input_mut().cursor_pos = state.input().input.len();
        }
        state.input_mut().file_picker_range_suffix = None;
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
        state.view_mut().dirty = true;
        return true;
    }
    if matches!(event, crate::Event::Quit) {
        *state.should_quit_mut() = true;
        return true;
    }
    false
}

fn is_palette_activation(dialog: &DialogState, event: &Event) -> bool {
    matches!(event, crate::Event::Submit | crate::Event::PaletteSelect)
        && matches!(
            dialog,
            DialogState::Active {
                kind: DialogKind::CommandPalette,
                panels: _
            }
        )
}

fn restore_or_pop_dialog(
    state: &mut AppState,
    dialog: DialogState,
    result: PanelUpdateResult,
    is_palette_activation: bool,
) {
    if result != PanelUpdateResult::Closed && state.open_dialog().is_none() {
        if is_palette_activation {
            state.dialog_back_stack_mut().pop();
        } else {
            *state.open_dialog_mut() = Some(dialog);
        }
    }
}

/// Push a dialog onto the global back stack.
pub fn push_dialog_to_back_stack(state: &mut AppState, dialog: DialogState) {
    state.push_dialog_to_back_stack(dialog);
}

/// Process the result of executing a command.
/// Closes the command palette after most command results (except when opening a new dialog).
pub fn process_command_result(state: &mut AppState, result: CommandResult) {
    use crate::commands::CommandResult as CR;
    match result {
        CR::Message(msg) => {
            // Close command palette and return to chat input
            close_command_palette_if_open(state);
            state.add_system_msg(msg);
        }
        CR::Warning(msg) => {
            // Close command palette and return to chat input
            close_command_palette_if_open(state);
            state.notify(msg, crate::event::TransientLevel::Warning);
        }
        CR::Event(evt) => {
            // Close command palette before processing the event
            // (the event may open a new dialog)
            close_command_palette_if_open(state);
            apply_command_event_for_dialog(state, evt);
        }
        CR::Events(evts) => {
            close_command_palette_if_open(state);
            for evt in evts {
                apply_command_event_for_dialog(state, evt);
            }
        }
        CR::OpenDialog(d) => {
            if let Some(current) = state.open_dialog_mut().take() {
                push_dialog_to_back_stack(state, current);
            }
            match d {
                DialogType::CommandPalette => open_command_palette(state),
                DialogType::ModelSelector => open_model_selector(state),
                DialogType::Settings => open_settings_dialog(state),
                DialogType::ScopedModels => open_scoped_models_dialog(state),
                DialogType::ThemeSelector => open_theme_selector(state),
            }
        }
        CR::OpenPanelStack(stack) => {
            if let Some(current) = state.open_dialog_mut().take() {
                push_dialog_to_back_stack(state, current);
            }
            state.view_mut().dirty = true;
            *state.open_dialog_mut() = Some(DialogState::Active {
                kind: DialogKind::Generic,
                panels: *stack,
            });
        }
        CR::None => {
            // No-op, close any open command palette
            close_command_palette_if_open(state);
        }
    }
}

/// Close the command palette if it is open, returning to chat input.
fn close_command_palette_if_open(state: &mut AppState) {
    if let Some(DialogState::Active { kind, .. }) = state.open_dialog() {
        if *kind == DialogKind::CommandPalette {
            *state.open_dialog_mut() = None;
            state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
            state.view_mut().scroll = 0;
            state.view_mut().dirty = true;
        }
    }
}

/// Apply a command-result event: add confirmation message AND process state change.
/// Mirrors `submit::apply_command_event` so both the dialog and submit paths
/// produce consistent UX (system messages for model/thinking/reset commands).
fn apply_command_event_for_dialog(state: &mut AppState, evt: crate::Event) {
    match &evt {
        crate::Event::SwitchModel { provider, model, .. } => {
            state.add_system_msg(format!("Switched to {}/{}", provider, model));
        }
        crate::Event::SetThinkingLevel(level) => {
            use crate::ui_strings::model as m;
            state.add_system_msg(m::thinking_level(level.as_str()));
        }
        crate::Event::NewSession => {
            state.add_system_msg(crate::ui_strings::session::NEW_SESSION_STARTED.into());
        }
        _ => {}
    }
    state.update(evt);
}
