//! Dialog Toggle Event Handler

use crate::commands::DialogState;
use crate::model::AppState;
use crate::Event;

use super::dialog;

pub fn dialog_toggle_event(state: &mut AppState, event: Event) {
    match event {
        Event::ToggleCommandPalette => dialog::open_command_palette(state),
        Event::ToggleModelSelector => dialog::toggle_dialog(
            state,
            matches!(state.open_dialog, Some(DialogState::ModelSelector(_))),
            dialog::open_model_selector,
        ),
        Event::ToggleScopedModelsDialog => dialog::toggle_dialog(
            state,
            matches!(state.open_dialog, Some(DialogState::ScopedModels(_))),
            dialog::open_scoped_models_dialog,
        ),
        Event::ToggleSettingsDialog => dialog::toggle_dialog(
            state,
            matches!(state.open_dialog, Some(DialogState::Settings(_))),
            dialog::open_settings_dialog,
        ),
        Event::ToggleSessionTree => dialog::toggle_dialog(
            state,
            matches!(state.open_dialog, Some(DialogState::SessionTree(_))),
            dialog::open_session_tree_dialog,
        ),
        Event::AtFilePicker => dialog::open_at_file_picker_all(state),
        Event::ScopedModelToggle { name } => {
            crate::update::scoped_models::toggle_scoped_model(state, &name)
        }
        Event::ScopedModelEnableAll => crate::update::scoped_models::enable_all(state),
        Event::ScopedModelDisableAll => crate::update::scoped_models::disable_all(state),
        Event::ScopedModelToggleProvider { provider } => {
            crate::update::scoped_models::toggle_provider(state, &provider)
        }
        _ => {}
    }
}
