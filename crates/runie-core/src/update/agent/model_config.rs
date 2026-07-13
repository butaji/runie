use crate::actors::SessionMsg;
use crate::model::AppState;
use crate::update::dialog::dialog_toggle_event;
use crate::Event;

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
            // Always switch during session replay (when restoring a session's model).
            // Skip only when non-explicit switch would override a user's explicit choice.
            // During replay, the session's model should always be restored.
            if *explicit || state.config().model_source != crate::model::ModelSource::UserOverride {
                state.switch_model(provider.clone(), model.clone(), *explicit);
            }
            true
        }
        crate::Event::SelectModel { provider, model } => {
            // Model picked in the `/model` selector: open the per-model
            // reasoning-level panel. The emitted SwitchModelWithLevel performs
            // the actual switch once a level is chosen.
            let key = format!("{provider}/{model}");
            let global = state.config().thinking_level;
            let override_level = state.config().model_thinking.get(&key).copied();
            let v = state.view_mut();
            v.input_receiver = crate::model::InputReceiver::Dialog;
            v.dirty = true;
            *state.open_dialog_mut() = Some(crate::commands::DialogState::Active {
                kind: crate::commands::DialogKind::ModelSelector,
                panels: crate::dialog::builders::model_reasoning_panel(
                    provider,
                    model,
                    global,
                    override_level,
                ),
            });
            true
        }
        crate::Event::SwitchModelWithLevel {
            provider,
            model,
            level,
        } => {
            state.set_model_thinking_level(provider, model, *level);
            state.switch_model(provider.clone(), model.clone(), true);
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

fn handle_trust_project(state: &mut AppState, decision: crate::trust::TrustDecision) {
    use crate::event::TransientLevel;
    let cwd = std::env::current_dir().unwrap_or_default();
    let cwd_utf8 =
        camino::Utf8PathBuf::from_path_buf(cwd).unwrap_or_else(|_| camino::Utf8PathBuf::from("."));
    // Update state synchronously (mirrors TrustActor logic for unit test compatibility).
    // TrustActor also processes this async for persistence.
    state.set_trust_decision(cwd_utf8.clone(), decision);
    let new_read_only = !matches!(decision, crate::trust::TrustDecision::Trusted);
    state.config_mut().read_only = new_read_only;
    // Remove welcome message and notify when trusted
    if matches!(decision, crate::trust::TrustDecision::Trusted) {
        state
            .session_mut()
            .messages
            .retain(|m| m.id != "trust_welcome");
        state.messages_changed();
        state.notify(
            format!("Project '{}' trusted. Read-only disabled.", cwd_utf8),
            TransientLevel::Success,
        );
    } else {
        state.notify(
            format!("Project '{}' untrusted. Read-only enabled.", cwd_utf8),
            TransientLevel::Warning,
        );
    }
    // Also send to SessionActor async for persistence
    if let Some(handles) = state.actor_handles() {
        let handles = handles.clone();
        let cwd_async = cwd_utf8;
        let _ = handles.session.try_send(SessionMsg::SetTrust {
            path: cwd_async,
            decision,
        });
    }
}

fn handle_scoped_events(state: &mut AppState, event: &crate::Event) -> bool {
    match event {
        crate::Event::TrustProject => {
            handle_trust_project(state, crate::trust::TrustDecision::Trusted);
            false
        }
        crate::Event::UntrustProject => {
            handle_trust_project(state, crate::trust::TrustDecision::Untrusted);
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
