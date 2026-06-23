//! Login Flow Event Handlers
//!
//! Handles the provider picker → key input → model selector workflow.
//! Manages its own dialog state via `LoginFlowState` and the global
//! back stack for Android-like ESC semantics.

use crate::dialog::{Panel, PanelStack};
use crate::event::LoginFlowEvent;
use crate::login_flow::{
    build_key_input, build_login_root, build_model_selector, build_provider_picker,
    build_validating_panel, LoginFlowState, LoginStep,
};

/// Top-level login flow dispatcher.
pub fn login_flow_event(state: &mut crate::model::AppState, event: LoginFlowEvent) {
    match event {
        LoginFlowEvent::Start => login_flow_start(state),
        LoginFlowEvent::SelectProvider { provider } => login_flow_select_provider(state, provider),
        LoginFlowEvent::SubmitKey { provider, key } => login_flow_submit_key(state, provider, key),
        LoginFlowEvent::ModelsFetched { models, .. } => {
            login_flow_validation_success(state, models)
        }
        LoginFlowEvent::ValidationFailed { error, .. } => {
            login_flow_validation_failed(state, error)
        }
        LoginFlowEvent::ToggleModel { model } => login_flow_toggle_model(state, model),
        LoginFlowEvent::Save => login_flow_save(state),
        LoginFlowEvent::Cancel => login_flow_cancel(state),
        _ => {}
    }
}

pub(crate) fn login_flow_start(state: &mut crate::model::AppState) {
    state.login_flow = Some(LoginFlowState::new());
    rebuild_login_dialog(state);
}

fn login_flow_select_provider(state: &mut crate::model::AppState, provider: String) {
    if let Some(ref mut flow) = state.login_flow {
        *flow = flow.clone().with_provider(provider);
        state.mark_dirty();
    }
    // Push the key input panel onto the real login stack (root + pushed).
    // ESC / Cancel will pop back to the provider picker.
    let panel = build_key_input(state.login_flow.as_ref().unwrap());
    push_login_panel(state, panel);
}

fn reject_empty_key(
    state: &mut crate::model::AppState,
    provider: &str,
    key: &str,
) -> Option<String> {
    if key.trim().is_empty() {
        let p = if provider.is_empty() {
            state
                .login_flow
                .as_ref()
                .map(|f| f.provider.clone())
                .unwrap_or_default()
        } else {
            provider.to_string()
        };
        state.set_transient(
            "API key is required.".into(),
            crate::event::TransientLevel::Warning,
        );
        return Some(p);
    }
    None
}

fn login_flow_submit_key(state: &mut crate::model::AppState, provider: String, key: String) {
    if reject_empty_key(state, &provider, &key).is_some() {
        let panel = build_key_input(state.login_flow.as_ref().unwrap());
        replace_top_login_panel_with(state, panel);
        return;
    }

    let final_provider = if provider.is_empty() {
        state
            .login_flow
            .as_ref()
            .map(|f| f.provider.clone())
            .unwrap_or_default()
    } else {
        provider
    };
    if let Some(ref mut flow) = state.login_flow {
        flow.provider = final_provider.clone();
        *flow = flow.clone().with_key(key.clone());
    }
    push_login_panel(state, build_validating_panel(&final_provider));
}

fn login_flow_validation_success(state: &mut crate::model::AppState, models: Vec<String>) {
    let Some(flow) = state.login_flow.clone() else {
        return;
    };
    if flow.step != LoginStep::Validating {
        return;
    }
    let updated = flow.with_validation_success(models);
    state.login_flow = Some(updated.clone());
    replace_top_login_panel_with(state, build_model_selector(&updated));
    state.mark_dirty();
}

fn login_flow_validation_failed(state: &mut crate::model::AppState, error: String) {
    let Some(flow) = state.login_flow.clone() else {
        return;
    };
    if flow.step != LoginStep::Validating {
        return;
    }
    state.set_transient(
        format!("Could not verify key: {}", error),
        crate::event::TransientLevel::Warning,
    );
    let updated = flow.with_validation_error();
    state.login_flow = Some(updated);
    pop_login_panel(state);
    state.mark_dirty();
}

fn login_flow_toggle_model(state: &mut crate::model::AppState, model: String) {
    if let Some(ref mut flow) = state.login_flow {
        flow.toggle_model(&model);
        // The form handler already flipped the checkbox value in place,
        // so there is no need to rebuild the panel (which would reset the
        // selection index).
        state.mark_dirty();
    }
}

fn provider_base_url(state: &crate::model::AppState, provider: &str) -> String {
    state
        .config_cache
        .as_ref()
        .and_then(|c| c.model_providers.get(provider))
        .filter(|p| !p.base_url.is_empty())
        .map(|p| p.base_url.clone())
        .unwrap_or_else(|| {
            crate::provider_registry::find_provider(provider)
                .map(|p| p.base_url.to_string())
                .unwrap_or_default()
        })
}

fn login_flow_save(state: &mut crate::model::AppState) {
    let Some(flow) = take_login_flow_if_ready(state) else {
        reopen_login_panel_if_flow_present(state);
        return;
    };

    if !persist_login_flow(state, &flow) {
        return;
    }

    activate_first_selected_model_if_none_active(state, &flow);
    close_login_flow(state);
}

fn reopen_login_panel_if_flow_present(state: &mut crate::model::AppState) {
    // Save was rejected (e.g. no models selected). Reopen the login dialog
    // at the current step so the user can correct the input.
    let Some(flow) = state.login_flow.as_ref() else {
        return;
    };
    let panel = match flow.step {
        LoginStep::KeyInput => build_key_input(flow),
        LoginStep::ModelSelect => build_model_selector(flow),
        _ => build_model_selector(flow),
    };
    replace_top_login_panel_with(state, panel);
}

fn persist_login_flow(
    state: &mut crate::model::AppState,
    flow: &crate::login_flow::LoginFlowState,
) -> bool {
    let base_url = provider_base_url(state, &flow.provider);
    let selected: Vec<String> = flow.selected_models.iter().cloned().collect();
    if let Some(tx) = state.config_tx.clone() {
        // Update config_cache immediately so downstream code (e.g. model selector,
        // /model command) sees the new provider without waiting for ConfigActor.
        sync_config_cache(state, &flow.provider, &base_url, &flow.key, &selected);
        let msg = crate::actors::ConfigMsg::SaveProvider {
            name: flow.provider.clone(),
            base_url,
            api_key: flow.key.clone(),
            models: selected,
        };
        tokio::spawn(async move {
            let _ = tx.send(msg).await;
        });
        true
    } else if let Err(e) =
        crate::login_config::save_provider_config(&flow.provider, &base_url, &flow.key, &selected)
    {
        state.add_system_msg(format!("Failed to save provider config: {}", e));
        false
    } else {
        // Sync the saved provider into config_cache so downstream code
        // (e.g. open_model_selector) sees it immediately without a reload.
        sync_config_cache(state, &flow.provider, &base_url, &flow.key, &selected);
        true
    }
}

fn sync_config_cache(
    state: &mut crate::model::AppState,
    provider: &str,
    base_url: &str,
    api_key: &str,
    models: &[String],
) {
    let cache = state.config_cache.get_or_insert_with(Default::default);
    cache.model_providers.insert(
        provider.into(),
        crate::config::ModelProvider {
            provider_type: cache
                .model_providers
                .get(provider)
                .and_then(|p| p.provider_type.clone()),
            base_url: base_url.into(),
            api_key: api_key.into(),
            models: models.into(),
        },
    );
}

fn activate_first_selected_model_if_none_active(
    state: &mut crate::model::AppState,
    flow: &crate::login_flow::LoginFlowState,
) {
    // Only switch to the newly saved provider when no provider/model is
    // currently active. This keeps the existing selection when the user is
    // adding another provider through the providers dialog.
    if !state.has_models() {
        activate_first_selected_model(state, flow);
    }
}

fn close_login_flow(state: &mut crate::model::AppState) {
    state.login_flow = None;
    state.dialog_back_stack.clear();
    state.open_dialog = None;
    state.view.input_receiver = crate::model::InputReceiver::ChatInput;
    state.mark_dirty();
}

fn take_login_flow_if_ready(state: &mut crate::model::AppState) -> Option<LoginFlowState> {
    let flow = state.login_flow.clone()?;
    if !flow.validated {
        state.set_transient(
            "Please wait for the API key to be validated before saving.".into(),
            crate::event::TransientLevel::Warning,
        );
        return None;
    }
    if flow.selected_models.is_empty() {
        state.set_transient(
            "Select at least one model before saving.".into(),
            crate::event::TransientLevel::Warning,
        );
        return None;
    }
    Some(flow)
}

fn activate_first_selected_model(state: &mut crate::model::AppState, flow: &LoginFlowState) {
    let first_model = flow
        .available_models
        .iter()
        .find(|m| flow.selected_models.contains(*m))
        .or_else(|| flow.selected_models.iter().next())
        .cloned()
        .unwrap_or_default();
    state.switch_model(flow.provider.clone(), first_model, false);
}

pub fn login_flow_cancel(state: &mut crate::model::AppState) {
    // Cancel pops one level. At the root (provider picker), the pop
    // is a no-op and we close the dialog.
    state.view.cached_auth_valid = false;
    pop_login_panel_or_close(state);
}

/// Pop the top panel of the login stack. If we're at the root, close
/// the entire dialog (and clear `login_flow`). The pop also updates
/// `LoginFlowState::step` to reflect the panel we returned to.
fn pop_login_panel_or_close(state: &mut crate::model::AppState) {
    if state.login_flow.is_none() {
        return;
    }
    let mut stack = take_or_create_login_stack(state);
    if stack.len() > 1 {
        stack.pop();
        // Update step to reflect the panel we returned to.
        if let Some(flow) = state.login_flow.as_mut() {
            flow.step = match stack.current().map(|p| p.id.as_str()) {
                Some("login-provider") => LoginStep::ProviderPicker,
                Some("login-key") => LoginStep::KeyInput,
                Some("login-validating") => LoginStep::Validating,
                Some("login-models") => LoginStep::ModelSelect,
                _ => flow.step.clone(),
            };
        }
        state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
        state.mark_dirty();
    } else if stack.root().map(|p| p.closable).unwrap_or(true) {
        // At the root: close the login flow and restore the previous
        // dialog from the back stack.
        state.login_flow = None;
        if let Some(previous) = state.dialog_back_stack.pop() {
            state.open_dialog = Some(previous);
            state.mark_dirty();
        } else {
            state.open_dialog = None;
            state.view.input_receiver = crate::model::InputReceiver::ChatInput;
            state.mark_dirty();
        }
    } else {
        // The root panel is marked non-closable: keep it open.
        state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
        state.mark_dirty();
    }
}

/// Pop the top panel of the login stack without closing the dialog.
/// Updates `LoginFlowState::step` to reflect the panel we returned to.
fn pop_login_panel(state: &mut crate::model::AppState) {
    if state.login_flow.is_none() {
        return;
    }
    let mut stack = take_or_create_login_stack(state);
    if stack.len() > 1 {
        stack.pop();
    }
    if let Some(flow) = state.login_flow.as_mut() {
        flow.step = match stack.current().map(|p| p.id.as_str()) {
            Some("login-provider") => LoginStep::ProviderPicker,
            Some("login-key") => LoginStep::KeyInput,
            Some("login-validating") => LoginStep::Validating,
            Some("login-models") => LoginStep::ModelSelect,
            _ => flow.step.clone(),
        };
    }
    state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
    state.mark_dirty();
}

/// Push a panel onto the login stack (and set the step on the state).
fn push_login_panel(state: &mut crate::model::AppState, panel: Panel) {
    if let Some(flow) = state.login_flow.as_mut() {
        flow.step = match panel.id.as_str() {
            "login-provider" => LoginStep::ProviderPicker,
            "login-key" => LoginStep::KeyInput,
            "login-validating" => LoginStep::Validating,
            "login-models" => LoginStep::ModelSelect,
            _ => flow.step.clone(),
        };
    }
    let mut stack = take_or_create_login_stack(state);
    stack.push(panel);
    state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
    state.mark_dirty();
}

/// Replace the top panel of the login stack with `new_top`, popping
/// the current top first. Used when a panel is "consumed" (e.g. the
/// key input is submitted → model selector).
fn replace_top_login_panel_with(state: &mut crate::model::AppState, new_top: Panel) {
    let mut stack = take_or_create_login_stack(state);
    if !stack.is_empty() {
        stack.pop();
    }
    stack.push(new_top);
    state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
    state.mark_dirty();
}

/// Take the current login PanelStack out of `open_dialog`, or build a
/// fresh root stack if there is no dialog.
///
/// When the dialog has been temporarily taken out of `open_dialog` (e.g.
/// while a dialog event is being processed), this reconstructs the full
/// stack from the current `LoginFlowState` so that nested updates such as
/// toggling a model or pressing Cancel still operate on the correct panels.
fn take_or_create_login_stack(state: &mut crate::model::AppState) -> PanelStack {
    if let Some(crate::commands::DialogState::PanelStack(stack)) = state.open_dialog.take() {
        return stack;
    }
    if let Some(flow) = state.login_flow.as_ref() {
        return build_login_stack_for_flow(flow);
    }
    build_login_root()
}

/// Reconstruct the login panel stack that corresponds to the current flow step.
fn build_login_stack_for_flow(flow: &LoginFlowState) -> PanelStack {
    let mut stack = build_login_root();
    if flow.step == LoginStep::ProviderPicker {
        return stack;
    }
    stack.push(build_key_input(flow));
    match flow.step {
        LoginStep::Validating => stack.push(build_validating_panel(&flow.provider)),
        LoginStep::ModelSelect => stack.push(build_model_selector(flow)),
        _ => {}
    }
    stack
}

fn rebuild_login_dialog(state: &mut crate::model::AppState) {
    // Open the login dialog with the root panel (provider picker).
    // If another dialog is open, push it onto the global back stack.
    if state.login_flow.is_some() {
        if let Some(current) = state.open_dialog.take() {
            state.dialog_back_stack.push(current);
        }
        let mut root = build_provider_picker();
        root.closable = state.has_models();
        let stack = PanelStack::new(root);
        state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
        state.mark_dirty();
    }
}

#[cfg(test)]
mod tests;
