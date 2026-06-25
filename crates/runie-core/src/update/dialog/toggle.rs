//! Dialog Toggle Event Handler (merged from dialog_toggle.rs).

use crate::commands::DialogState;
use crate::model::AppState;

use super::{
    open_at_file_picker_all, open_command_palette, open_model_selector, open_scoped_models_dialog,
    open_settings_dialog,
};

pub fn dialog_toggle_event(state: &mut AppState, event: crate::Event) {
    match &event {
        crate::Event::ToggleWelcome => handle_welcome_toggle(state),
        crate::Event::ToggleCommandPalette => open_command_palette(state),
        crate::Event::ToggleSettingsDialog => handle_settings_toggle(state),
        crate::Event::ToggleModelSelector => handle_model_selector_toggle(state),
        crate::Event::AtFilePicker => open_at_file_picker_all(state),
        crate::Event::ToggleVimMode => handle_vim_mode_toggle(state),
        crate::Event::TogglePathCompletion => state.toggle_path_completion(),
        crate::Event::PathCompletionUp => state.path_completion_up(),
        crate::Event::PathCompletionDown => state.path_completion_down(),
        crate::Event::PathCompletionSelect => state.path_completion_select(),
        crate::Event::PathCompletionClose => state.path_completion_close(),
        crate::Event::ProvidersDialog => handle_providers_dialog(state),
        crate::Event::ProvidersAdd => handle_providers_add(state),
        crate::Event::ProvidersSelectModel { .. } => handle_providers_select_model(state, &event),
        crate::Event::ProvidersDisconnect { .. } => handle_providers_disconnect(state, &event),
        crate::Event::ProvidersEditModels { .. } => handle_providers_edit_models(state, &event),

        crate::Event::ToggleScopedModelsDialog => handle_scoped_models_toggle(state),
        crate::Event::ScopedModelEnableAll => handle_scoped_model_enable_all(state),
        crate::Event::ScopedModelDisableAll => handle_scoped_model_disable_all(state),
        // intentionally ignored: other crate::Event variants fall through
        _ => {}
    }
}

fn handle_welcome_toggle(state: &mut AppState) {
    let is_welcome = matches!(state.open_dialog(), Some(&DialogState::Welcome));
    *state.open_dialog_mut() = if is_welcome {
        None
    } else {
        Some(DialogState::Welcome)
    };
    state.view_mut().dirty = true;
}

fn handle_model_selector_toggle(state: &mut AppState) {
    do_toggle_dialog(
        state,
        matches!(state.open_dialog(), Some(&DialogState::ModelSelector(_))),
        open_model_selector,
    );
}

fn handle_scoped_models_toggle(state: &mut AppState) {
    do_toggle_dialog(
        state,
        matches!(state.open_dialog(), Some(&DialogState::ScopedModels(_))),
        open_scoped_models_dialog,
    );
}

fn handle_settings_toggle(state: &mut AppState) {
    do_toggle_dialog(
        state,
        matches!(state.open_dialog(), Some(&DialogState::Settings(_))),
        open_settings_dialog,
    );
}

fn handle_vim_mode_toggle(state: &mut AppState) {
    state.config_mut().vim_mode = !state.config().vim_mode;
    state.view_mut().cached_settings_valid = false;
}

fn handle_providers_dialog(state: &mut AppState) {
    use crate::provider::dialog::build_providers_dialog;
    *state.open_dialog_mut() = Some(DialogState::PanelStack(build_providers_dialog(state)));
    state.view_mut().dirty = true;
}

fn handle_providers_add(state: &mut AppState) {
    // Hand off to the login flow machinery, which pushes the current dialog to
    // the back stack and opens the provider picker. The root panel is marked
    // non-closable when no model is connected so the user cannot cancel out.
    crate::login_flow::login_flow_start(state);
}

fn handle_providers_select_model(state: &mut AppState, event: &crate::Event) {
    if let crate::Event::ProvidersSelectModel { provider, model } = event {
        if let Some(mut flow) = state.login_flow_mut().take() {
            flow.selected_models.insert(model.clone());
            *state.login_flow_mut() = Some(flow);
        }
        state.switch_model(provider.clone(), model.clone(), true);
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
        state.dialog_back_stack_mut().clear();
        state.view_mut().dirty = true;
    }
}

fn handle_providers_edit_models(state: &mut AppState, event: &crate::Event) {
    if let crate::Event::ProvidersEditModels { provider } = event {
        let stack = crate::provider::dialog::build_provider_models_editor(state, provider);
        if let Some(DialogState::PanelStack(current)) = state.open_dialog_mut().as_mut() {
            if let Some(panel) = stack.current() {
                current.push(panel.clone());
            }
        } else {
            *state.open_dialog_mut() = Some(DialogState::PanelStack(stack));
        }
        state.view_mut().dirty = true;
    }
}

fn handle_providers_disconnect(state: &mut AppState, event: &crate::Event) {
    if let crate::Event::ProvidersDisconnect { provider } = event {
        let provider = provider.clone();
        // Fire-and-forget async removal (no-op in tests without ConfigActor).
        state.remove_provider(&provider);
        // Also sync config_cache directly so tests and sync paths see the change immediately.
        if let Some(cache) = state.config_cache_mut() {
            cache.model_providers.remove(&provider);
        }
        if state.config().current_provider == provider {
            let (provider, model) = state.resolve_default_model();
            state.set_active_model(provider, model, crate::model::ModelSource::ConfigDefault);
        }
        if state.has_models() {
            *state.open_dialog_mut() = None;
            state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
        } else {
            crate::login_flow::login_flow_start(state);
        }
        state.dialog_back_stack_mut().clear();
        state.view_mut().dirty = true;
    }
}

fn handle_scoped_model_enable_all(state: &mut AppState) {
    set_scoped_models_enabled(state, true);
}

fn handle_scoped_model_disable_all(state: &mut AppState) {
    set_scoped_models_enabled(state, false);
}

fn set_scoped_models_enabled(state: &mut AppState, enabled: bool) {
    for model in &mut state.config_mut().scoped_models {
        model.enabled = enabled;
    }
    state.view_mut().dirty = true;
}

fn do_toggle_dialog(state: &mut AppState, is_same: bool, open: fn(&mut AppState)) {
    if is_same {
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
        state.view_mut().dirty = true;
    } else {
        open(state);
    }
}
