use crate::Event;
use crate::model::AppState;
use crate::update::dialog::dialog_toggle_event;

pub fn model_config_event(state: &mut AppState, event: crate::Event) {
    let invalidate = handle_main_events(state, &event)
        || handle_scoped_events(state, &event)
        || handle_settings_events(state, &event);
    if invalidate {
        state.view_mut().cached_settings_valid = false;
    }
}

fn handle_main_events(state: &mut AppState, event: &crate::Event) -> bool {
    match event {
        crate::Event::SwitchModel {
            provider,
            model,
            explicit,
        } => {
            if *explicit || state.config().model_source != crate::model::ModelSource::UserOverride {
                state.switch_model(provider.clone(), model.clone(), *explicit);
            }
            true
        }
        crate::Event::SwitchTheme { name } => {
            state.switch_theme(name.clone());
            true
        }
        crate::Event::CycleModelNext => {
            state.cycle_model(1);
            false
        }
        crate::Event::CycleModelPrev => {
            state.cycle_model(-1);
            false
        }
        crate::Event::CycleThinkingLevel => {
            state.cycle_thinking_level();
            true
        }
        crate::Event::SetThinkingLevel(level) => {
            state.set_thinking_level(*level);
            true
        }
        crate::Event::ToggleReadOnly => {
            state.toggle_read_only();
            true
        }
        _ => false,
    }
}

fn handle_scoped_events(state: &mut AppState, event: &crate::Event) -> bool {
    match event {
        crate::Event::TrustProject => {
            state.apply_trust_project();
            false
        }
        crate::Event::UntrustProject => {
            state.apply_untrust_project();
            false
        }
        crate::Event::ReloadAll => {
            // Reload is now owned by ConfigActor; this event is kept for
            // backward compatibility with old session replays.
            false
        }
        crate::Event::ScopedModelToggle { provider, name } => {
            super::scoped_models::toggle_scoped_model(state, provider, name);
            false
        }
        crate::Event::ScopedModelEnableAll => {
            super::scoped_models::enable_all(state);
            false
        }
        crate::Event::ScopedModelDisableAll => {
            super::scoped_models::disable_all(state);
            false
        }
        crate::Event::ScopedModelToggleProvider { provider } => {
            super::scoped_models::toggle_provider(state, provider);
            false
        }
        _ => false,
    }
}

/// Handle settings dialog navigation and selection events.
/// When a dialog is open, delegate to update_dialog for proper panel stack handling.
fn handle_settings_events(state: &mut AppState, event: &crate::Event) -> bool {
    match event {
        crate::Event::ToggleSettingsDialog => {
            dialog_toggle_event(state, Event::ToggleSettingsDialog);
            true
        }
        crate::Event::ToggleScopedModelsDialog => {
            dialog_toggle_event(state, Event::ToggleScopedModelsDialog);
            true
        }
        crate::Event::SettingsClose => {
            crate::update::dialog::update_dialog(state, event.clone());
            true
        }
        crate::Event::SettingsSelect
        | crate::Event::SettingsDown
        | crate::Event::SettingsUp
        | crate::Event::SettingsLeft
        | crate::Event::SettingsRight => {
            if state.open_dialog().is_some() {
                crate::update::dialog::update_dialog(state, event.clone());
            }
            true
        }
        _ => false,
    }
}
