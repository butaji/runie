//! Dialog toggle event handlers (merged from toggle.rs, provider_model_toggle.rs, model_selector.rs).

use crate::commands::DialogState;
use crate::model::AppState;

use super::{
    open_at_file_picker_all, open_command_palette, open_model_selector, open_scoped_models_dialog,
    open_settings_dialog,
};

// ---------------------------------------------------------------------------
// Public API (re-exports for back-compat via mod.rs)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Dialog toggle events
// ---------------------------------------------------------------------------

/// Route a dialog-toggle event to its handler.
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
        _ => {}
    }
}

fn handle_welcome_toggle(state: &mut AppState) {
    let is_welcome = matches!(state.open_dialog(), Some(&DialogState::Welcome));
    *state.open_dialog_mut() = if is_welcome { None } else { Some(DialogState::Welcome) };
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
    let new_value = !state.config().vim_mode;
    state.config_mut().vim_mode = new_value;
    // Persist to config.toml via ConfigActor (clone handles for async)
    let handles = state.actor_handles().cloned();
    if let Some(h) = handles {
        if tokio::runtime::Handle::try_current().is_ok() {
            let h = h;
            tokio::spawn(async move {
                h.send_set_vim_mode(new_value).await;
            });
        }
    }
    state.view_mut().cached_settings_valid = false;
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

// ---------------------------------------------------------------------------
// Scoped model helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Provider dialog handlers
// ---------------------------------------------------------------------------

fn handle_providers_dialog(state: &mut AppState) {
    use crate::provider::dialog::build_providers_dialog;
    *state.open_dialog_mut() = Some(DialogState::PanelStack(build_providers_dialog(state)));
    state.view_mut().dirty = true;
}

fn handle_providers_add(state: &mut AppState) {
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
        state.remove_provider(&provider);
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

// ---------------------------------------------------------------------------
// Provider model toggle (from settings dialog)
// ---------------------------------------------------------------------------

/// Parse a toggle key produced by the settings dialog for a provider/model.
/// Keys have the form `edit_provider:<provider>:<model>`.
pub fn parse_provider_model_toggle(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix("edit_provider:")?;
    rest.split_once(':')
}

/// Toggle whether `model` is enabled for `provider` in the saved config.
pub fn toggle_provider_model(state: &mut AppState, provider: &str, model: &str) {
    let provider = provider.to_owned();
    let model = model.to_owned();
    let current_models: Vec<String> = state
        .provider_config(&provider)
        .map(|p| p.models)
        .or_else(|| crate::login_config::get_provider_config(&provider).map(|(_, _, m)| m))
        .unwrap_or_default();
    let mut models = current_models;
    let pos = models.iter().position(|m| m == &model);
    if let Some(idx) = pos {
        models.remove(idx);
    } else {
        models.push(model.clone());
        models.sort();
    }
    sync_provider_models(state, &provider, &models);
    state.set_provider_models(&provider, models.clone());
    if provider == state.config().current_provider && !models.contains(&model) {
        if let Some(first) = models.first() {
            state.switch_model(provider.clone(), first.clone(), false);
        }
    }
    state.view_mut().cached_settings_valid = false;
}

fn sync_provider_models(state: &mut AppState, provider: &str, models: &[String]) {
    // Update local cache for immediate UI feedback
    if let Some(cache) = state.config_cache_mut() {
        let (base_url, api_key) = crate::login_config::get_provider_config(provider)
            .map(|(b, k, _)| (b, k))
            .unwrap_or_else(|| {
                (
                    crate::provider::find_provider(provider)
                        .map(|p| p.base_url.to_owned())
                        .unwrap_or_default(),
                    String::new(),
                )
            });
        cache
            .model_providers
            .entry(provider.into())
            .or_insert_with(|| crate::config::ModelProvider {
                provider_type: None,
                base_url,
                api_key,
                models: vec![],
            })
            .models = models.to_vec();
    }
    // Persist to config.toml via ConfigActor
    let handles = state.actor_handles().cloned();
    if let Some(h) = handles {
        let provider = provider.to_owned();
        let models = models.to_vec();
        if tokio::runtime::Handle::try_current().is_ok() {
            let h = h;
            tokio::spawn(async move {
                h.send_set_provider_models(&provider, models).await;
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Model selector helpers
// ---------------------------------------------------------------------------

/// Partition model catalog items into recent models and provider groups.
#[allow(clippy::type_complexity)]
pub fn partition_model_items(
    items: Vec<(String, String, String, bool, bool)>,
) -> (Vec<String>, Vec<(String, Vec<(String, crate::Event)>)>) {
    let mut recent: Vec<String> = Vec::new();
    let mut groups: Vec<(String, Vec<(String, crate::Event)>)> = Vec::new();
    let mut last_header = String::new();
    let mut current_group: Vec<(String, crate::Event)> = Vec::new();
    for (header, name, _cost, _is_selected, _is_current) in items {
        if header == "Recent" {
            recent.push(name);
            continue;
        }
        if !header.is_empty() && header != last_header {
            if !current_group.is_empty() {
                groups.push((last_header.clone(), std::mem::take(&mut current_group)));
            }
            last_header = header.clone();
        }
        if let Some((provider, model)) = name.split_once('/') {
            let evt = crate::Event::SwitchModel {
                provider: provider.to_owned(),
                model: model.to_owned(),
                explicit: true,
            };
            current_group.push((name, evt));
        }
    }
    if !current_group.is_empty() {
        groups.push((last_header, current_group));
    }
    (recent, groups)
}
