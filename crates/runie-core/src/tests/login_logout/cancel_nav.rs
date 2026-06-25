use crate::login_flow::LoginStep;
use crate::model::AppState;
use crate::Event;

use super::{
    add_provider_and_select_model, assert_panel_id, assert_step, clean_config, current_panel_id,
    fetch_models, select_provider, start_login_flow, submit_key,
};

fn disconnected_state() -> AppState {
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state
}

#[test]
fn cancel_at_key_input_returns_to_provider_picker() {
    clean_config();
    let mut state = disconnected_state();
    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    assert_step(&state, LoginStep::KeyInput);

    state.update(Event::from(crate::Event::Cancel));

    assert_step(&state, LoginStep::ProviderPicker);
    assert_panel_id(&state, "login-provider");
}

#[test]
fn cancel_at_model_select_returns_to_key_input() {
    clean_config();
    let mut state = disconnected_state();
    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".to_string()]);
    assert_step(&state, LoginStep::ModelSelect);

    state.update(Event::from(crate::Event::Cancel));

    assert_step(&state, LoginStep::KeyInput);
    assert_panel_id(&state, "login-key");
}

#[test]
fn cancel_at_provider_picker_blocked_without_model() {
    clean_config();
    let mut state = disconnected_state();
    start_login_flow(&mut state);
    assert!(state.login_flow.is_some());

    state.update(Event::from(crate::Event::Cancel));

    assert!(
        state.login_flow.is_some(),
        "login flow should remain active"
    );
    assert!(state.open_dialog.is_some(), "login dialog should stay open");
    assert_step(&state, LoginStep::ProviderPicker);
    assert_panel_id(&state, "login-provider");
}

#[test]
fn cancel_at_provider_picker_allowed_with_model() {
    clean_config();
    let mut state = disconnected_state();
    add_provider_and_select_model(&mut state, "minimax", "sk-test", "MiniMax-M3");

    start_login_flow(&mut state);
    state.update(Event::from(crate::Event::Cancel));

    assert!(state.login_flow.is_none(), "login flow should be closed");
    assert_ne!(
        current_panel_id(&state).as_deref(),
        Some("login-provider"),
        "login provider picker should be closed"
    );
}

#[test]
fn dialog_back_behaves_like_cancel() {
    clean_config();
    let mut state = disconnected_state();
    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".to_string()]);
    assert_step(&state, LoginStep::ModelSelect);

    state.update(Event::from(crate::Event::DialogBack));

    assert_step(&state, LoginStep::KeyInput);
    assert_panel_id(&state, "login-key");
}

#[test]
fn abort_behaves_like_cancel() {
    clean_config();
    let mut state = disconnected_state();
    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".to_string()]);
    assert_step(&state, LoginStep::ModelSelect);

    state.update(Event::from(crate::Event::Abort));

    assert_step(&state, LoginStep::KeyInput);
    assert_panel_id(&state, "login-key");
}

#[test]
fn force_quit_always_closes() {
    clean_config();
    let mut state = disconnected_state();
    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");

    state.update(Event::from(crate::Event::ForceQuit));

    assert!(state.should_quit, "ForceQuit should set should_quit");
}

#[test]
fn esc_at_root_blocked_without_model() {
    clean_config();
    let mut state = disconnected_state();
    start_login_flow(&mut state);
    assert!(state.login_flow.is_some());

    state.update(Event::from(crate::Event::DialogBack));

    assert!(
        state.login_flow.is_some(),
        "login flow should remain active"
    );
    assert!(state.open_dialog.is_some(), "login dialog should stay open");
    assert_step(&state, LoginStep::ProviderPicker);
    assert_panel_id(&state, "login-provider");
}
