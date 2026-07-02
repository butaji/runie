//! Login Flow Event Handlers
//!
//! Handles the provider picker → key input → model selector workflow.
//! Manages its own dialog state via `LoginFlowState` and the global
//! back stack for Android-like ESC semantics.

use super::panels::{build_key_input, build_model_selector, build_validating_panel};
use super::state::{LoginFlowState, LoginStep};
use crate::actors::ConfigMsg;
use crate::login_flow::panel_ops::{
    pop_login_panel, pop_login_panel_or_close, push_login_panel, rebuild_login_dialog,
    replace_top_login_panel_with,
};
use crate::Event;

/// Top-level login flow dispatcher.
pub fn login_flow_event(state: &mut crate::model::AppState, event: Event) {
    match event {
        Event::Start => login_flow_start(state),
        Event::SelectProvider { provider } => login_flow_select_provider(state, provider),
        Event::SubmitKey { provider, key } => login_flow_submit_key(state, provider, key),
        Event::ModelsFetched { models, .. } => login_flow_validation_success(state, models),
        Event::ValidationFailed { error, .. } => login_flow_validation_failed(state, error),
        Event::ToggleModel { model } => login_flow_toggle_model(state, model),
        Event::Save => login_flow_save(state),
        Event::Cancel => login_flow_cancel(state),
        // intentionally ignored: other LoginFlow variants are not used in this handler
        _ => {}
    }
}

pub fn login_flow_start(state: &mut crate::model::AppState) {
    *state.login_flow_mut() = Some(LoginFlowState::new());
    rebuild_login_dialog(state);
}

fn login_flow_select_provider(state: &mut crate::model::AppState, provider: String) {
    let current_key = state
        .login_flow
        .as_ref()
        .map(|f| f.key.clone())
        .unwrap_or_default();

    if let Some(flow) = state.login_flow_mut() {
        *flow = LoginFlowState {
            step: LoginStep::KeyInput,
            provider,
            key: current_key,
            available_models: std::mem::take(&mut flow.available_models),
            selected_models: std::mem::take(&mut flow.selected_models),
            validated: false,
        };
        state.view_mut().dirty = true;
    }
    let panel = build_key_input(state.login_flow().as_ref().unwrap());
    push_login_panel(state, panel);
}

fn reject_empty_key(
    state: &mut crate::model::AppState,
    provider: &str,
    key: &str,
) -> Option<String> {
    if key.trim().is_empty() {
        let p = if provider.is_empty() {
            state.active_provider()
        } else {
            provider.to_owned()
        };
        state.warn("API key is required.");
        return Some(p);
    }
    None
}

fn login_flow_submit_key(state: &mut crate::model::AppState, provider: String, key: String) {
    if reject_empty_key(state, &provider, &key).is_some() {
        let panel = build_key_input(state.login_flow().as_ref().unwrap());
        replace_top_login_panel_with(state, panel);
        return;
    }

    let final_provider = if provider.is_empty() {
        state.active_provider()
    } else {
        provider
    };

    let available_models = state
        .login_flow
        .as_mut()
        .map(|f| std::mem::take(&mut f.available_models))
        .unwrap_or_default();
    let selected_models = state
        .login_flow
        .as_mut()
        .map(|f| std::mem::take(&mut f.selected_models))
        .unwrap_or_default();

    if let Some(flow) = state.login_flow_mut() {
        flow.provider = final_provider.clone();
        flow.key = key.clone();
        flow.step = LoginStep::Validating;
        flow.available_models = available_models;
        flow.selected_models = selected_models;
        flow.validated = false;
    }
    push_login_panel(state, build_validating_panel(&final_provider));
}

fn login_flow_validation_success(state: &mut crate::model::AppState, models: Vec<String>) {
    let current_step = state.login_flow().as_ref().map(|f| f.step.clone());
    if current_step.as_ref() != Some(&LoginStep::Validating) {
        return;
    }

    let selected_models: std::collections::HashSet<String> = models.iter().cloned().collect();
    if let Some(flow) = state.login_flow_mut() {
        flow.step = LoginStep::ModelSelect;
        flow.available_models = models;
        flow.selected_models = selected_models;
        flow.validated = true;
    }

    transition_to_model_selector(state);
}

fn transition_to_model_selector(state: &mut crate::model::AppState) {
    let provider = state.active_provider();
    let key = state
        .login_flow
        .as_ref()
        .map(|f| f.key.clone())
        .unwrap_or_default();
    let updated = LoginFlowState {
        step: LoginStep::ModelSelect,
        provider,
        key,
        available_models: state
            .login_flow
            .as_ref()
            .map(|f| f.available_models.clone())
            .unwrap_or_default(),
        selected_models: state
            .login_flow
            .as_ref()
            .map(|f| f.selected_models.clone())
            .unwrap_or_default(),
        validated: true,
    };
    replace_top_login_panel_with(state, build_model_selector(&updated));
    state.view_mut().dirty = true;
}

fn login_flow_validation_failed(state: &mut crate::model::AppState, error: String) {
    let current_step = state.login_flow().as_ref().map(|f| f.step.clone());
    if current_step.as_ref() != Some(&LoginStep::Validating) {
        return;
    }

    let available_models = state
        .login_flow
        .as_mut()
        .map(|f| std::mem::take(&mut f.available_models))
        .unwrap_or_default();
    let selected_models = state
        .login_flow
        .as_mut()
        .map(|f| std::mem::take(&mut f.selected_models))
        .unwrap_or_default();

    state.warn(format!("Could not verify key: {}", error));
    if let Some(flow) = state.login_flow_mut() {
        flow.step = LoginStep::KeyInput;
        flow.available_models = available_models;
        flow.selected_models = selected_models;
        flow.validated = false;
    }
    pop_login_panel(state);
    state.view_mut().dirty = true;
}

fn login_flow_toggle_model(state: &mut crate::model::AppState, model: String) {
    if let Some(flow) = state.login_flow_mut() {
        flow.toggle_model(&model);
        state.view_mut().dirty = true;
    }
}

pub(crate) fn provider_base_url(state: &crate::model::AppState, provider: &str) -> String {
    state
        .config()
        .model_providers()
        .get(provider)
        .filter(|p| !p.base_url.is_empty())
        .map(|p| p.base_url.clone())
        .unwrap_or_else(|| {
            crate::provider::find_provider(provider)
                .map(|p| p.base_url.to_owned())
                .unwrap_or_default()
        })
}

fn login_flow_save(state: &mut crate::model::AppState) {
    if !validate_login_flow_ready(state) {
        return;
    }

    let (provider, key, selected, base_url) = extract_login_flow_data(state);
    let has_current_models = state.has_models();

    persist_provider_config(state, &provider, &base_url, &key, &selected);

    if !has_current_models {
        activate_first_model(state, &provider, &selected);
    }

    close_login_flow(state);
}

fn validate_login_flow_ready(state: &mut crate::model::AppState) -> bool {
    let validated = state
        .login_flow()
        .as_ref()
        .map(|f| f.validated)
        .unwrap_or(false);
    let has_models = state
        .login_flow
        .as_ref()
        .map(|f| !f.selected_models.is_empty())
        .unwrap_or(false);

    if !validated {
        state.warn("Please wait for the API key to be validated before saving.");
        reopen_login_panel_if_flow_present(state);
        return false;
    }
    if !has_models {
        state.warn("Select at least one model before saving.");
        reopen_login_panel_if_flow_present(state);
        return false;
    }
    true
}

fn extract_login_flow_data(
    state: &crate::model::AppState,
) -> (String, String, Vec<String>, String) {
    let provider = state.active_provider();
    let key = state
        .login_flow
        .as_ref()
        .map(|f| f.key.clone())
        .unwrap_or_default();
    let selected: Vec<String> = state
        .login_flow
        .as_ref()
        .map(|f| f.selected_models.iter().cloned().collect())
        .unwrap_or_default();
    let base_url = provider_base_url(state, &provider);
    (provider, key, selected, base_url)
}

fn persist_provider_config(
    state: &mut crate::model::AppState,
    provider: &str,
    base_url: &str,
    key: &str,
    selected: &[String],
) {
    sync_config_cache(state, provider, base_url, key, selected);

    // Route through ConfigActor in production; fall back to direct save in tests.
    if let Some(h) = state.actor_handles() {
        let provider = provider.to_owned();
        let base_url = base_url.to_owned();
        let key = key.to_owned();
        let selected = selected.to_vec();
        let _ = h.config.try_send(ConfigMsg::SaveProvider {
            name: provider,
            base_url,
            api_key: key,
            models: selected,
        });
        return;
    }
    if let Err(e) = crate::provider::config::save_provider_config(provider, base_url, key, selected)
    {
        state.add_system_msg(format!("Failed to save provider config: {}", e));
    }
}

fn activate_first_model(state: &mut crate::model::AppState, provider: &str, selected: &[String]) {
    let first_model = state
        .login_flow
        .as_ref()
        .and_then(|f| {
            f.available_models
                .iter()
                .find(|m| f.selected_models.contains(*m))
                .or_else(|| selected.iter().next())
                .cloned()
        })
        .unwrap_or_default();
    state.switch_model(provider.to_owned(), first_model, false);
}

fn sync_config_cache(
    state: &mut crate::model::AppState,
    provider: &str,
    base_url: &str,
    api_key: &str,
    models: &[String],
) {
    // Store api_key in keyring (never in config)
    if !api_key.is_empty() {
        if let Err(e) = crate::auth::set_keyring_value(provider, api_key) {
            tracing::warn!("failed to store api_key in keyring: {}", e);
        }
    }

    let providers = state.config_mut().model_providers_mut();
    providers.insert(
        provider.into(),
        crate::config::ModelProvider {
            provider_type: providers
                .get(provider)
                .and_then(|p| p.provider_type.clone()),
            base_url: base_url.into(),
            models: models.into(),
        },
    );
}

fn reopen_login_panel_if_flow_present(state: &mut crate::model::AppState) {
    let Some(flow) = state.login_flow() else {
        return;
    };
    let panel = match flow.step {
        LoginStep::KeyInput => build_key_input(flow),
        LoginStep::ModelSelect => build_model_selector(flow),
        _ => build_model_selector(flow),
    };
    replace_top_login_panel_with(state, panel);
}

pub fn login_flow_cancel(state: &mut crate::model::AppState) {
    state.view_mut().cached_auth_valid = false;
    pop_login_panel_or_close(state);
}

fn close_login_flow(state: &mut crate::model::AppState) {
    *state.login_flow_mut() = None;
    state.dialog_back_stack_mut().clear();
    *state.open_dialog_mut() = None;
    state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
    state.view_mut().dirty = true;
}
