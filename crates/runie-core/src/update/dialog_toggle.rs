//! Dialog Toggle Event Handler

use crate::commands::DialogState;
use crate::model::AppState;
use crate::Event;

use super::dialog_stack::{self, open_command_palette, open_model_selector,
    open_scoped_models_dialog, open_settings_dialog, open_session_tree_dialog};

pub fn dialog_toggle_event(state: &mut AppState, event: Event) {
    match event {
        Event::ToggleCommandPalette => open_command_palette(state),
        Event::ToggleModelSelector => dialog_stack::toggle_dialog(
            state,
            matches!(state.open_dialog, Some(DialogState::ModelSelector(_))),
            open_model_selector,
        ),
        Event::ToggleScopedModelsDialog => dialog_stack::toggle_dialog(
            state,
            matches!(state.open_dialog, Some(DialogState::ScopedModels(_))),
            open_scoped_models_dialog,
        ),
        Event::ToggleSettingsDialog => dialog_stack::toggle_dialog(
            state,
            matches!(state.open_dialog, Some(DialogState::Settings(_))),
            open_settings_dialog,
        ),
        Event::ToggleSessionTree => dialog_stack::toggle_dialog(
            state,
            matches!(state.open_dialog, Some(DialogState::SessionTree(_))),
            open_session_tree_dialog,
        ),
        Event::AtFilePicker => dialog_stack::open_at_file_picker(state),
        Event::ScopedModelToggle { name } => crate::update::scoped_models::toggle_scoped_model(state, &name),
        Event::ScopedModelEnableAll => crate::update::scoped_models::enable_all(state),
        Event::ScopedModelDisableAll => crate::update::scoped_models::disable_all(state),
        Event::ScopedModelToggleProvider { provider } => {
            crate::update::scoped_models::toggle_provider(state, &provider)
        }
        _ => {}
    }
}
