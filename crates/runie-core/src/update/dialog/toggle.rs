//! Dialog Toggle Event Handler (merged from dialog_toggle.rs).

use crate::commands::DialogState;
use crate::event::DialogEvent;
use crate::model::AppState;

use super::{
    open_at_file_picker_all, open_command_palette, open_model_selector, open_scoped_models_dialog,
    open_settings_dialog,
};

pub fn dialog_toggle_event(state: &mut AppState, event: DialogEvent) {
    match &event {
        DialogEvent::ToggleWelcome => handle_welcome_toggle(state),
        DialogEvent::ToggleCommandPalette => open_command_palette(state),
        DialogEvent::ToggleSettingsDialog => handle_settings_toggle(state),
        DialogEvent::ToggleModelSelector => handle_model_selector_toggle(state),
        DialogEvent::AtFilePicker => open_at_file_picker_all(state),
        DialogEvent::ToggleVimMode => handle_vim_mode_toggle(state),
        DialogEvent::TogglePathCompletion => state.toggle_path_completion(),
        DialogEvent::PathCompletionUp => state.path_completion_up(),
        DialogEvent::PathCompletionDown => state.path_completion_down(),
        DialogEvent::PathCompletionSelect => state.path_completion_select(),
        DialogEvent::PathCompletionClose => state.path_completion_close(),
        DialogEvent::ProvidersDialog => handle_providers_dialog(state),
        DialogEvent::ProvidersAdd => handle_providers_add(state),
        DialogEvent::ProvidersSelectModel { .. } => handle_providers_select_model(state, &event),
        DialogEvent::ProvidersDisconnect { .. } => handle_providers_disconnect(state, &event),

        DialogEvent::ToggleScopedModelsDialog => handle_scoped_models_toggle(state),
        DialogEvent::ScopedModelEnableAll => handle_scoped_model_enable_all(state),
        DialogEvent::ScopedModelDisableAll => handle_scoped_model_disable_all(state),
        _ => {}
    }
}

fn handle_welcome_toggle(state: &mut AppState) {
    let is_welcome = matches!(state.open_dialog, Some(DialogState::Welcome));
    state.open_dialog = if is_welcome {
        None
    } else {
        Some(DialogState::Welcome)
    };
    state.mark_dirty();
}

fn handle_model_selector_toggle(state: &mut AppState) {
    do_toggle_dialog(
        state,
        matches!(state.open_dialog, Some(DialogState::ModelSelector(_))),
        open_model_selector,
    );
}

fn handle_scoped_models_toggle(state: &mut AppState) {
    do_toggle_dialog(
        state,
        matches!(state.open_dialog, Some(DialogState::ScopedModels(_))),
        open_scoped_models_dialog,
    );
}

fn handle_settings_toggle(state: &mut AppState) {
    do_toggle_dialog(
        state,
        matches!(state.open_dialog, Some(DialogState::Settings(_))),
        open_settings_dialog,
    );
}

fn handle_vim_mode_toggle(state: &mut AppState) {
    state.config.vim_mode = !state.config.vim_mode;
    state.view.cached_settings_valid = false;
}

fn handle_providers_dialog(state: &mut AppState) {
    use crate::providers_dialog::build_providers_dialog;
    state.open_dialog = Some(DialogState::PanelStack(build_providers_dialog(state)));
    state.mark_dirty();
}

fn handle_providers_add(state: &mut AppState) {
    // Hand off to the login flow machinery, which pushes the current dialog to
    // the back stack and opens the provider picker. The root panel is marked
    // non-closable when no model is connected so the user cannot cancel out.
    crate::update::login_flow::login_flow_start(state);
}

fn handle_providers_select_model(state: &mut AppState, event: &DialogEvent) {
    if let DialogEvent::ProvidersSelectModel { provider, model } = event {
        if let Some(mut flow) = state.login_flow.take() {
            flow.selected_models.insert(model.clone());
            state.login_flow = Some(flow);
        }
        state.switch_model(provider.clone(), model.clone(), true);
        state.open_dialog = None;
        state.view.input_receiver = crate::model::InputReceiver::ChatInput;
        state.dialog_back_stack.clear();
        state.mark_dirty();
    }
}

fn handle_providers_disconnect(state: &mut AppState, event: &DialogEvent) {
    if let DialogEvent::ProvidersDisconnect { provider } = event {
        let provider = provider.clone();
        // Fire-and-forget async removal (no-op in tests without ConfigActor).
        state.remove_provider(&provider);
        // Also sync config_cache directly so tests and sync paths see the change immediately.
        if let Some(ref mut cache) = state.config_cache {
            cache.model_providers.remove(&provider);
        }
        if state.config.current_provider == provider {
            let (provider, model) = state.resolve_default_model();
            state.set_active_model(
                provider,
                model,
                crate::state::ModelSource::ConfigDefault,
            );
        }
        if state.has_models() {
            state.open_dialog = None;
            state.view.input_receiver = crate::model::InputReceiver::ChatInput;
        } else {
            crate::update::login_flow::login_flow_start(state);
        }
        state.dialog_back_stack.clear();
        state.mark_dirty();
    }
}

fn handle_scoped_model_enable_all(state: &mut AppState) {
    for model in &mut state.config.scoped_models {
        model.enabled = true;
    }
    state.mark_dirty();
}

fn handle_scoped_model_disable_all(state: &mut AppState) {
    for model in &mut state.config.scoped_models {
        model.enabled = false;
    }
    state.mark_dirty();
}

fn do_toggle_dialog(state: &mut AppState, is_same: bool, open: fn(&mut AppState)) {
    if is_same {
        state.open_dialog = None;
        state.view.input_receiver = crate::model::InputReceiver::ChatInput;
        state.mark_dirty();
    } else {
        open(state);
    }
}
