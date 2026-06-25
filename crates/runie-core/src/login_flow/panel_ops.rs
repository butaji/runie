//! Login flow panel operations — push, pop, replace, rebuild.

use super::panels::{
    build_key_input, build_login_root, build_model_selector, build_provider_picker,
    build_validating_panel,
};
use super::state::LoginFlowState;
use crate::dialog::{Panel, PanelStack};

/// Push a panel onto the login stack (and set the step on the state).
pub(super) fn push_login_panel(state: &mut crate::model::AppState, panel: Panel) {
    if let Some(flow) = state.login_flow_mut().as_mut() {
        flow.step = match panel.id.as_str() {
            "login-provider" => super::state::LoginStep::ProviderPicker,
            "login-key" => super::state::LoginStep::KeyInput,
            "login-validating" => super::state::LoginStep::Validating,
            "login-models" => super::state::LoginStep::ModelSelect,
            _ => flow.step.clone(),
        };
    }
    let mut stack = take_or_create_login_stack(state);
    stack.push(panel);
    *state.open_dialog_mut() = Some(crate::commands::DialogState::PanelStack(stack));
    state.view_mut().dirty = true;
}

/// Replace the top panel of the login stack with `new_top`, popping
/// the current top first. Used when a panel is "consumed" (e.g. the
/// key input is submitted → model selector).
pub(super) fn replace_top_login_panel_with(state: &mut crate::model::AppState, new_top: Panel) {
    let mut stack = take_or_create_login_stack(state);
    if !stack.is_empty() {
        stack.pop();
    }
    stack.push(new_top);
    *state.open_dialog_mut() = Some(crate::commands::DialogState::PanelStack(stack));
    state.view_mut().dirty = true;
}

/// Pop the top panel of the login stack without closing the dialog.
/// Updates `LoginFlowState::step` to reflect the panel we returned to.
pub(super) fn pop_login_panel(state: &mut crate::model::AppState) {
    if state.login_flow().is_none() {
        return;
    }
    let mut stack = take_or_create_login_stack(state);
    if stack.len() > 1 {
        stack.pop();
    }
    if let Some(flow) = state.login_flow_mut().as_mut() {
        flow.step = match stack.current().map(|p| p.id.as_str()) {
            Some("login-provider") => super::state::LoginStep::ProviderPicker,
            Some("login-key") => super::state::LoginStep::KeyInput,
            Some("login-validating") => super::state::LoginStep::Validating,
            Some("login-models") => super::state::LoginStep::ModelSelect,
            _ => flow.step.clone(),
        };
    }
    *state.open_dialog_mut() = Some(crate::commands::DialogState::PanelStack(stack));
    state.view_mut().dirty = true;
}

/// Pop the top panel of the login stack. If we're at the root, close
/// the entire dialog (and clear `login_flow`). The pop also updates
/// `LoginFlowState::step` to reflect the panel we returned to.
pub(super) fn pop_login_panel_or_close(state: &mut crate::model::AppState) {
    if state.login_flow().is_none() {
        return;
    }
    let mut stack = take_or_create_login_stack(state);
    if stack.len() > 1 {
        stack.pop();
        if let Some(flow) = state.login_flow_mut().as_mut() {
            flow.step = match stack.current().map(|p| p.id.as_str()) {
                Some("login-provider") => super::state::LoginStep::ProviderPicker,
                Some("login-key") => super::state::LoginStep::KeyInput,
                Some("login-validating") => super::state::LoginStep::Validating,
                Some("login-models") => super::state::LoginStep::ModelSelect,
                _ => flow.step.clone(),
            };
        }
        *state.open_dialog_mut() = Some(crate::commands::DialogState::PanelStack(stack));
        state.view_mut().dirty = true;
    } else if stack.root().map(|p| p.closable).unwrap_or(true) {
        // At the root: close the login flow and restore the previous
        // dialog from the back stack.
        *state.login_flow_mut() = None;
        if let Some(previous) = state.dialog_back_stack_mut().pop() {
            *state.open_dialog_mut() = Some(previous);
            state.view_mut().dirty = true;
        } else {
            *state.open_dialog_mut() = None;
            state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
            state.view_mut().dirty = true;
        }
    } else {
        // The root panel is marked non-closable: keep it open.
        *state.open_dialog_mut() = Some(crate::commands::DialogState::PanelStack(stack));
        state.view_mut().dirty = true;
    }
}

/// Take the current login PanelStack out of `open_dialog`, or build a
/// fresh root stack if there is no dialog.
///
/// When the dialog has been temporarily taken out of `open_dialog` (e.g.
/// while a dialog event is being processed), this reconstructs the full
/// stack from the current `LoginFlowState` so that nested updates such as
/// toggling a model or pressing Cancel still operate on the correct panels.
fn take_or_create_login_stack(state: &mut crate::model::AppState) -> PanelStack {
    if let Some(crate::commands::DialogState::PanelStack(stack)) = state.open_dialog_mut().take() {
        return stack;
    }
    if let Some(flow) = state.login_flow().as_ref() {
        return build_login_stack_for_flow(flow);
    }
    build_login_root()
}

/// Reconstruct the login panel stack that corresponds to the current flow step.
fn build_login_stack_for_flow(flow: &LoginFlowState) -> PanelStack {
    let mut stack = build_login_root();
    if flow.step == super::state::LoginStep::ProviderPicker {
        return stack;
    }
    stack.push(build_key_input(flow));
    match flow.step {
        super::state::LoginStep::Validating => stack.push(build_validating_panel(&flow.provider)),
        super::state::LoginStep::ModelSelect => stack.push(build_model_selector(flow)),
        // intentionally ignored: other login steps are handled elsewhere
        _ => {}
    }
    stack
}

/// Open the login dialog with the root panel (provider picker).
/// If another dialog is open, push it onto the global back stack.
pub(super) fn rebuild_login_dialog(state: &mut crate::model::AppState) {
    if state.login_flow().is_some() {
        if let Some(current) = state.open_dialog_mut().take() {
            state.dialog_back_stack_mut().push(current);
        }
        let mut root = build_provider_picker();
        root.closable = state.has_models();
        let stack = PanelStack::new(root);
        *state.open_dialog_mut() = Some(crate::commands::DialogState::PanelStack(stack));
        state.view_mut().dirty = true;
    }
}
