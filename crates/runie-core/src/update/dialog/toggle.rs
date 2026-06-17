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
        DialogEvent::OpenAgentsManager
        | DialogEvent::AgentsManagerSetField { .. }
        | DialogEvent::AgentsManagerSave { .. }
        | DialogEvent::AgentsManagerDelete { .. } => handle_agents_manager_event(state, &event),
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

fn handle_agents_manager_event(state: &mut AppState, event: &DialogEvent) {
    crate::commands::agents_manager::agents_manager_event(state, event.clone());
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
    state.open_dialog = Some(DialogState::PanelStack(build_providers_dialog(
        &state.config.current_provider,
        &state.config.current_model,
    )));
    state.mark_dirty();
}

fn handle_providers_add(state: &mut AppState) {
    if let Some(current) = state.open_dialog.take() {
        state.push_dialog_to_back_stack(current);
    }
    state.login_flow = Some(crate::login_flow::LoginFlowState::new());
    state.mark_dirty();
}

fn handle_providers_select_model(state: &mut AppState, event: &DialogEvent) {
    if let DialogEvent::ProvidersSelectModel { provider, model } = event {
        if let Some(mut flow) = state.login_flow.take() {
            flow.selected_models.insert(model.clone());
            state.login_flow = Some(flow);
        }
        state.config.current_provider = provider.clone();
        state.config.current_model = model.clone();
        state.configure_token_tracker();
        state.record_model_usage(provider, model);
        state.open_dialog = None;
        state.dialog_back_stack.clear();
        state.mark_dirty();
    }
}

fn handle_providers_disconnect(state: &mut AppState, event: &DialogEvent) {
    if let DialogEvent::ProvidersDisconnect { provider } = event {
        let _ = crate::login_config::remove_provider_config(provider);
        if state.config.current_provider == *provider {
            let configured = crate::login_config::list_configured_providers();
            if let Some((name, _, models)) = configured.first() {
                state.config.current_provider = name.clone();
                state.config.current_model = models.first().cloned().unwrap_or_default();
            } else {
                state.config.current_provider.clear();
                state.config.current_model.clear();
            }
            state.configure_token_tracker();
        }
        if state.has_models() {
            state.open_dialog = None;
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
        state.mark_dirty();
    } else {
        open(state);
    }
}
