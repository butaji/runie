//! Dialog routing, command-result processing, and back-stack helpers.

use crate::commands::{CommandResult, DialogState, DialogType};
use crate::event::ControlEvent;
use crate::model::AppState;
use crate::Event;

use super::{
    open_command_palette, open_model_selector, open_provider_models_dialog,
    open_scoped_models_dialog, open_settings_dialog,
    panel::{update_panel_stack, PanelUpdateResult},
};

/// Handles dialog-specific events. Returns whether the dialog was closed.
pub fn update_dialog(state: &mut AppState, event: Event) {
    if route_global_dialog_event(state, &event) {
        return;
    }
    let Some(mut dialog) = state.open_dialog.take() else {
        return;
    };
    // Welcome dialog has no panel stack — only close on specific events
    if matches!(dialog, crate::commands::DialogState::Welcome) {
        state.open_dialog = Some(dialog);
        return;
    }
    let is_dialog_back = matches!(&event, crate::event::DialogEvent::DialogBack);
    let is_palette_activation = is_palette_activation(&dialog, &event);
    if is_palette_activation {
        state.push_dialog_to_back_stack(dialog.clone());
    }
    let stack = dialog
        .panel_stack_mut()
        .expect("non-welcome dialog has panel stack");
    let result = update_panel_stack(state, event, stack);
    restore_or_pop_dialog(state, dialog, result, is_palette_activation);

    if is_dialog_back && state.open_dialog.is_none() {
        state.handle_vim_dialog_back();
    }

    state.mark_dirty();
}

fn route_global_dialog_event(state: &mut AppState, event: &Event) -> bool {
    if matches!(event, ControlEvent::Abort) {
        if let Some((input, _, _, _)) = state.input.file_picker_backup.take() {
            state.input.input = input;
            state.input.cursor_pos = state.input.input.len();
        }
        state.input.file_picker_range_suffix = None;
        state.open_dialog = None;
        state.mark_dirty();
        return true;
    }
    if matches!(event, ControlEvent::Quit) {
        state.should_quit = true;
        return true;
    }
    false
}

fn is_palette_activation(dialog: &DialogState, event: &Event) -> bool {
    use crate::event::{DialogEvent, InputEvent};
    matches!(event, InputEvent::Submit | DialogEvent::PaletteSelect)
        && matches!(dialog, DialogState::CommandPalette(_))
}

fn restore_or_pop_dialog(
    state: &mut AppState,
    dialog: DialogState,
    result: PanelUpdateResult,
    is_palette_activation: bool,
) {
    if result != PanelUpdateResult::Closed && state.open_dialog.is_none() {
        if is_palette_activation {
            state.dialog_back_stack.pop();
        } else {
            state.open_dialog = Some(dialog);
        }
    }
}

/// Push a dialog onto the global back stack.
pub fn push_dialog_to_back_stack(state: &mut AppState, dialog: DialogState) {
    state.push_dialog_to_back_stack(dialog);
}

/// Toggle a dialog open/closed.
#[allow(dead_code)]
pub fn toggle_dialog(state: &mut AppState, is_same: bool, open: fn(&mut AppState)) {
    if is_same {
        state.open_dialog = None;
        state.mark_dirty();
    } else {
        open(state);
    }
}

/// Process the result of executing a command.
pub fn process_command_result(state: &mut AppState, result: CommandResult) {
    use crate::commands::CommandResult as CR;
    match result {
        CR::Message(msg) => state.add_system_msg(msg),
        CR::Warning(msg) => state.notify(msg, crate::event::TransientLevel::Warning),
        CR::Event(evt) => state.update(evt),
        CR::OpenDialog(d) => {
            if let Some(current) = state.open_dialog.take() {
                push_dialog_to_back_stack(state, current);
            }
            match d {
                DialogType::CommandPalette => open_command_palette(state),
                DialogType::ModelSelector => open_model_selector(state),
                DialogType::ProviderModels => {
                    let provider = state.config.current_provider.clone();
                    if !provider.is_empty() {
                        open_provider_models_dialog(state, &provider);
                    }
                }
                DialogType::Settings => open_settings_dialog(state),
                DialogType::ScopedModels => open_scoped_models_dialog(state),
            }
        }
        CR::OpenPanelStack(stack) => {
            if let Some(current) = state.open_dialog.take() {
                push_dialog_to_back_stack(state, current);
            }
            state.open_dialog = Some(DialogState::PanelStack(*stack));
            state.mark_dirty();
        }
        CR::None => {}
    }
}
