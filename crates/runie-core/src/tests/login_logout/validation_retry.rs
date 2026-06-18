//! Validation failure and retry scenarios for the login flow.

use crate::event::LoginFlowEvent;
use crate::login_flow::LoginStep;
use crate::model::AppState;

use super::{
    assert_step, assert_transient_contains, clean_config, default_models_for_provider,
    save_login_flow, select_provider, submit_key,
};

#[test]
fn empty_key_rejected_stays_on_key_input() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(LoginFlowEvent::Start);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "   ");

    assert_step(&state, LoginStep::KeyInput);
    assert_transient_contains(&state, "API key is required");
}

#[test]
fn validation_failed_returns_to_key_input_with_error() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(LoginFlowEvent::Start);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    assert_step(&state, LoginStep::Validating);

    state.update(LoginFlowEvent::ValidationFailed {
        provider: "minimax".into(),
        key: "sk-test".into(),
        error: "bad key".into(),
    });

    assert_step(&state, LoginStep::KeyInput);
    assert_transient_contains(&state, "Could not verify key");
}

#[test]
fn retry_after_failure_succeeds() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(LoginFlowEvent::Start);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    state.update(LoginFlowEvent::ValidationFailed {
        provider: "minimax".into(),
        key: "sk-test".into(),
        error: "bad key".into(),
    });
    assert_step(&state, LoginStep::KeyInput);

    submit_key(&mut state, "sk-test2");
    assert_step(&state, LoginStep::Validating);

    let models = default_models_for_provider("minimax");
    state.update(LoginFlowEvent::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test2".into(),
        models,
    });
    assert_step(&state, LoginStep::ModelSelect);

    save_login_flow(&mut state);
    assert!(state.has_models(), "state should report a connected model");
}

#[test]
fn save_before_validation_rejected() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(LoginFlowEvent::Start);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    save_login_flow(&mut state);

    assert_step(&state, LoginStep::Validating);
    assert_transient_contains(&state, "Please wait for the API key to be validated");
}

#[test]
fn unknown_provider_validation_failure() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(LoginFlowEvent::Start);
    select_provider(&mut state, "not-a-real-provider");
    submit_key(&mut state, "sk-test");
    state.update(LoginFlowEvent::ValidationFailed {
        provider: "not-a-real-provider".into(),
        key: "sk-test".into(),
        error: "unknown provider".into(),
    });

    assert!(state.login_flow.is_some(), "login flow should stay active");
    assert_step(&state, LoginStep::KeyInput);
    assert_transient_contains(&state, "Could not verify key");
}

#[test]
fn models_fetched_ignored_when_not_validating() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(LoginFlowEvent::Start);
    state.update(LoginFlowEvent::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });
    assert_step(&state, LoginStep::ProviderPicker);

    select_provider(&mut state, "minimax");
    state.update(LoginFlowEvent::ModelsFetched {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });
    assert_step(&state, LoginStep::KeyInput);
}
