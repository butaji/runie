use crate::event::ModelConfigEvent;
use crate::model::AppState;
use crate::update::dialog::dialog_toggle_event;

pub fn model_config_event(state: &mut AppState, event: ModelConfigEvent) {
    let invalidate = handle_main_events(state, &event)
        || handle_scoped_events(state, &event)
        || handle_settings_events(state, &event);
    if invalidate {
        state.view.cached_settings_valid = false;
    }
}

fn handle_main_events(state: &mut AppState, event: &ModelConfigEvent) -> bool {
    match event {
        ModelConfigEvent::SwitchModel { provider, model } => {
            state.switch_model(provider.clone(), model.clone());
            true
        }
        ModelConfigEvent::SwitchTheme { name } => {
            state.switch_theme(name.clone());
            true
        }
        ModelConfigEvent::CycleModelNext => {
            state.cycle_model(1);
            false
        }
        ModelConfigEvent::CycleModelPrev => {
            state.cycle_model(-1);
            false
        }
        ModelConfigEvent::CycleThinkingLevel => {
            state.cycle_thinking_level();
            true
        }
        ModelConfigEvent::SetThinkingLevel(level) => {
            state.set_thinking_level(*level);
            true
        }
        ModelConfigEvent::ToggleReadOnly => {
            state.toggle_read_only();
            true
        }
        _ => false,
    }
}

fn handle_scoped_events(state: &mut AppState, event: &ModelConfigEvent) -> bool {
    match event {
        ModelConfigEvent::TrustProject => {
            state.trust_project();
            false
        }
        ModelConfigEvent::UntrustProject => {
            state.untrust_project();
            false
        }
        ModelConfigEvent::ReloadAll => {
            state.reload_all();
            false
        }
        ModelConfigEvent::ScopedModelToggle { name } => {
            super::scoped_models::toggle_scoped_model(state, name);
            false
        }
        ModelConfigEvent::ScopedModelEnableAll => {
            super::scoped_models::enable_all(state);
            false
        }
        ModelConfigEvent::ScopedModelDisableAll => {
            super::scoped_models::disable_all(state);
            false
        }
        ModelConfigEvent::ScopedModelToggleProvider { provider } => {
            super::scoped_models::toggle_provider(state, provider);
            false
        }
        _ => false,
    }
}

/// Handle settings dialog navigation and selection events.
/// When a dialog is open, delegate to update_dialog for proper panel stack handling.
fn handle_settings_events(state: &mut AppState, event: &ModelConfigEvent) -> bool {
    match event {
        ModelConfigEvent::ToggleSettingsDialog => {
            dialog_toggle_event(state, crate::event::DialogEvent::ToggleSettingsDialog);
            true
        }
        ModelConfigEvent::ToggleScopedModelsDialog => {
            dialog_toggle_event(state, crate::event::DialogEvent::ToggleScopedModelsDialog);
            true
        }
        ModelConfigEvent::SettingsClose => {
            crate::update::dialog::update_dialog(state, event.clone());
            true
        }
        ModelConfigEvent::SettingsSelect
        | ModelConfigEvent::SettingsDown
        | ModelConfigEvent::SettingsUp
        | ModelConfigEvent::SettingsLeft
        | ModelConfigEvent::SettingsRight => {
            if state.open_dialog.is_some() {
                crate::update::dialog::update_dialog(state, event.clone());
            }
            true
        }
        _ => false,
    }
}
