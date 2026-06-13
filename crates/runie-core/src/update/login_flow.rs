//! Login Flow Event Handlers
//!
//! Handles the provider picker → key input → model selector workflow.
//! Manages its own dialog state via `LoginFlowState` and the global
//! back stack for Android-like ESC semantics.

use crate::dialog::{Panel, PanelStack};
use crate::login_flow::{build_key_input, build_login_root, build_model_selector, build_provider_picker, LoginFlowState, LoginStep};

/// Event handler for providers dialog (not part of the login flow state machine).
/// Routes `ProvidersDialog`, `ProvidersSelectModel`, `ProvidersDisconnect`, `ProvidersAdd`.
pub fn providers_event(state: &mut crate::model::AppState, event: crate::Event) {
    match event {
        crate::Event::ProvidersDialog => open_providers_dialog(state),
        crate::Event::ProvidersSelectModel { provider, model } => {
            providers_select_model(state, &provider, &model);
        }
        crate::Event::ProvidersDisconnect { provider } => {
            providers_disconnect(state, &provider);
        }
        crate::Event::ProvidersAdd => {
            // Close the providers dialog and start the login flow.
            // Push current dialog to back stack so Esc returns here.
            if let Some(current) = state.open_dialog.take() {
                state.dialog_back_stack.push(current);
            }
            state.login_flow = Some(LoginFlowState::new());
            rebuild_login_dialog(state);
        }
        _ => {}
    }
}

fn open_providers_dialog(state: &mut crate::model::AppState) {
    use crate::providers_dialog::build_providers_dialog;
    // Save the current dialog (e.g. palette) to the back stack so Esc
    // can restore it after the providers dialog or login flow closes.
    if let Some(current) = state.open_dialog.take() {
        state.dialog_back_stack.push(current);
    }
    state.open_dialog = Some(crate::commands::DialogState::PanelStack(build_providers_dialog(
        &state.config.current_provider,
        &state.config.current_model,
    )));
    state.mark_dirty();
}

fn providers_select_model(state: &mut crate::model::AppState, provider: &str, model: &str) {
    state.config.current_provider = provider.to_string();
    state.config.current_model = model.to_string();
    state.record_model_usage(provider, model);
    state.open_dialog = None;
    state.mark_dirty();
}

fn providers_disconnect(state: &mut crate::model::AppState, provider: &str) {
    match crate::login_config::remove_provider_config(provider) {
        Ok(()) => {
            // If this was the active provider, switch to another one or clear.
            if state.config.current_provider == provider {
                // Try to find another configured provider.
                let configured = crate::login_config::list_configured_providers();
                if let Some((name, _, models)) = configured.first() {
                    state.config.current_provider = name.clone();
                    state.config.current_model = models.first().cloned().unwrap_or_default();
                } else {
                    state.config.current_provider.clear();
                    state.config.current_model.clear();
                }
            }
            state.open_dialog = None;
            state.mark_dirty();
        }
        Err(e) => {
            state.set_transient(
                format!("Could not disconnect {}: {}", provider, e),
                crate::event::TransientLevel::Error,
            );
        }
    }
}

/// Top-level login flow dispatcher.
pub fn login_flow_event(state: &mut crate::model::AppState, event: crate::Event) {
    match event {
        crate::Event::LoginFlowStart => login_flow_start(state),
        crate::Event::LoginFlowSelectProvider { provider } => {
            login_flow_select_provider(state, provider)
        }
        crate::Event::LoginFlowSubmitKey { provider, key } => {
            login_flow_submit_key(state, provider, key)
        }
        crate::Event::LoginFlowValidationDone { models, .. } => {
            login_flow_validation_done(state, models)
        }
        crate::Event::LoginFlowValidationFailed { error, .. } => {
            login_flow_validation_failed(state, error)
        }
        crate::Event::LoginFlowModelsFetched { models, .. } => {
            login_flow_models_fetched(state, models)
        }
        crate::Event::LoginFlowToggleModel { model } => login_flow_toggle_model(state, model),
        crate::Event::LoginFlowSave => login_flow_save(state),
        crate::Event::LoginFlowCancel => login_flow_cancel(state),
        _ => {}
    }
}

fn login_flow_start(state: &mut crate::model::AppState) {
    state.login_flow = Some(LoginFlowState::new());
    rebuild_login_dialog(state);
}

fn login_flow_select_provider(state: &mut crate::model::AppState, provider: String) {
    let provider_clone = provider.clone();
    if let Some(ref mut flow) = state.login_flow {
        *flow = flow.clone().with_provider(provider);
        state.mark_dirty();
    }
    // Push the key input panel onto the real login stack (root + pushed).
    // ESC / Cancel will pop back to the provider picker.
    push_login_panel(state, build_key_input(&provider_clone));
}

fn login_flow_submit_key(state: &mut crate::model::AppState, provider: String, key: String) {
    // Compute defaults + final provider first (immutable borrows).
    let final_provider = if provider.is_empty() {
        state
            .login_flow
            .as_ref()
            .map(|f| f.provider.clone())
            .unwrap_or_default()
    } else {
        provider.clone()
    };
    let defaults: Vec<String> = crate::provider_registry::find_provider(&final_provider)
        .map(|meta| meta.default_models.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    // Update state (mutable borrow).
    if let Some(ref mut flow) = state.login_flow {
        *flow = flow.clone().with_key_and_defaults(key, defaults);
        flow.provider = final_provider.clone();
    }
    // Replace the key input panel with the model selector on the
    // real login stack. The key input is "consumed" (submitted) —
    // it should NOT remain in the back stack, otherwise Esc from
    // the model selector would pop back to a stale key input.
    if let Some(flow) = state.login_flow.as_ref() {
        replace_top_login_panel_with(state, build_model_selector(flow));
    }
}

fn login_flow_validation_done(state: &mut crate::model::AppState, models: Vec<String>) {
    if let Some(ref mut flow) = state.login_flow {
        // Non-blocking: enrich the model list in place on the top
        // panel (model selector). We do NOT push a new panel.
        *flow = flow.clone().with_fetched_models(models);
        replace_top_login_panel(state);
        state.mark_dirty();
    }
}

fn login_flow_models_fetched(state: &mut crate::model::AppState, models: Vec<String>) {
    if let Some(ref mut flow) = state.login_flow {
        if flow.step == LoginStep::ModelSelect {
            *flow = flow.clone().with_fetched_models(models);
            replace_top_login_panel(state);
            state.mark_dirty();
        }
    }
}

fn login_flow_validation_failed(state: &mut crate::model::AppState, error: String) {
    // Non-blocking: surface a transient warning, do NOT change the step
    // or the panel stack.
    if let Some(ref flow) = state.login_flow {
        if flow.step == LoginStep::ModelSelect {
            state.set_transient(
                format!("Could not verify key: {}", error),
                crate::event::TransientLevel::Warning,
            );
            state.mark_dirty();
        }
    }
}

fn login_flow_toggle_model(state: &mut crate::model::AppState, model: String) {
    if let Some(ref mut flow) = state.login_flow {
        flow.toggle_model(&model);
        // Refresh the top panel to reflect the new toggle state.
        replace_top_login_panel(state);
        state.mark_dirty();
    }
}

fn login_flow_save(state: &mut crate::model::AppState) {
    let _provider = if let Some(ref flow) = state.login_flow {
        let base_url = crate::provider_registry::find_provider(&flow.provider)
            .map(|p| p.base_url.to_string())
            .unwrap_or_default();
        let selected: Vec<String> = flow.selected_models.iter().cloned().collect::<Vec<_>>();
        let provider = flow.provider.clone();
        if let Err(e) = crate::login_config::save_provider_config(
            &provider,
            &base_url,
            &flow.key,
            &selected,
        ) {
            state.add_system_msg(format!("Failed to save provider config: {}", e));
            return;
        }
        provider
    } else {
        return;
    };

    // Clear the login flow state.
    state.login_flow = None;

    // Restore the providers dialog (from back stack) so the user can
    // choose which model to activate.
    if let Some(previous) = state.dialog_back_stack.pop() {
        state.open_dialog = Some(previous);
    } else {
        // Fallback: open the providers dialog directly.
        open_providers_dialog(state);
    }

    state.mark_dirty();
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
                Some("login-models") => LoginStep::ModelSelect,
                _ => flow.step.clone(),
            };
        }
        state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
        state.mark_dirty();
    } else {
        // At the root: close the login flow and restore the previous
        // dialog from the back stack.
        state.login_flow = None;
        if let Some(previous) = state.dialog_back_stack.pop() {
            state.open_dialog = Some(previous);
            state.mark_dirty();
        } else {
            state.open_dialog = None;
            state.mark_dirty();
        }
    }
}

/// Push a panel onto the login stack (and set the step on the state).
fn push_login_panel(state: &mut crate::model::AppState, panel: Panel) {
    if let Some(flow) = state.login_flow.as_mut() {
        flow.step = match panel.id.as_str() {
            "login-provider" => LoginStep::ProviderPicker,
            "login-key" => LoginStep::KeyInput,
            "login-models" => LoginStep::ModelSelect,
            _ => flow.step.clone(),
        };
    }
    let mut stack = take_or_create_login_stack(state);
    stack.push(panel);
    state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
}

/// Replace the top panel of the login stack with a freshly built one
/// from the current `LoginFlowState`. Used to update the model
/// selector when models are fetched or a model is toggled.
fn replace_top_login_panel(state: &mut crate::model::AppState) {
    let flow = state.login_flow.as_ref().cloned();
    let Some(flow) = flow else {
        return;
    };
    let mut stack = take_or_create_login_stack(state);
    if let Some(last) = stack.panels.last_mut() {
        *last = match last.id.as_str() {
            "login-models" => build_model_selector(&flow),
            "login-key" => build_key_input(&flow.provider),
            "login-provider" => build_provider_picker(),
            _ => build_model_selector(&flow),
        };
    }
    state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
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
}

/// Take the current login PanelStack out of `open_dialog`, or build a
/// fresh root stack if there is no dialog.
fn take_or_create_login_stack(state: &mut crate::model::AppState) -> PanelStack {
    if let Some(crate::commands::DialogState::PanelStack(stack)) = state.open_dialog.take() {
        stack
    } else {
        build_login_root()
    }
}

fn rebuild_login_dialog(state: &mut crate::model::AppState) {
    // Open the login dialog with the root panel (provider picker).
    // If another dialog is open, push it onto the global back stack.
    if state.login_flow.is_some() {
        if let Some(current) = state.open_dialog.take() {
            state.dialog_back_stack.push(current);
        }
        let stack = build_login_root();
        state.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
        state.mark_dirty();
    }
}
