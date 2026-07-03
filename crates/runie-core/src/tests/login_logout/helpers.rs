//! Shared helpers for login/onboarding flow tests.

use crate::login_flow::LoginStep;
use crate::model::AppState;

pub fn start_login_flow(state: &mut AppState) {
    state.update(crate::Event::Start);
}

pub fn select_provider(state: &mut AppState, provider: &str) {
    state.update(crate::Event::SelectProvider {
        provider: provider.into(),
    });
}

pub fn submit_key(state: &mut AppState, key: &str) {
    state.update(crate::Event::SubmitKey {
        provider: String::new(),
        key: key.into(),
    });
}

pub fn fetch_models(state: &mut AppState, models: &[String]) {
    let flow = state
        .login_flow
        .as_ref()
        .expect("fetch_models requires an active login flow");
    state.update(crate::Event::ModelsFetched {
        provider: flow.provider.clone(),
        key: flow.key.clone(),
        models: models.to_vec(),
    });
}

pub fn fetch_models_for(state: &mut AppState, provider: &str, key: &str, models: &[String]) {
    state.update(crate::Event::ModelsFetched {
        provider: provider.into(),
        key: key.into(),
        models: models.to_vec(),
    });
}

pub fn save_login_flow(state: &mut AppState) {
    state.update(crate::Event::Save);
}

pub fn assert_step(state: &AppState, step: LoginStep) {
    let flow = state
        .login_flow
        .as_ref()
        .expect("assert_step requires an active login flow");
    assert_eq!(
        flow.step, step,
        "expected login step {:?}, got {:?}",
        step, flow.step
    );
}

pub fn assert_panel_id(state: &AppState, id: &str) {
    let actual = current_panel_id(state);
    assert_eq!(
        actual.as_deref(),
        Some(id),
        "expected current panel id {:?}, got {:?}",
        id,
        actual
    );
}

pub fn assert_transient_contains(state: &AppState, text: &str) {
    let message = state.transient_message.as_deref().unwrap_or("").to_string();
    assert!(
        message.contains(text),
        "expected transient message to contain {:?}, got {:?}",
        text,
        message
    );
}

pub fn current_panel_id(state: &AppState) -> Option<String> {
    state
        .open_dialog
        .as_ref()
        .and_then(|dialog| dialog.panel_stack())
        .and_then(|stack| stack.current())
        .map(|panel| panel.id.clone())
}

pub fn current_panel(state: &AppState) -> Option<crate::dialog::Panel> {
    state
        .open_dialog
        .as_ref()
        .and_then(|dialog| dialog.panel_stack())
        .and_then(|stack| stack.current())
        .cloned()
}
