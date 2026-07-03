//! Tests for domain_ops.

#[allow(unused_imports)]
use crate::model::AppState;

#[test]
fn active_provider_returns_login_flow_provider() {
    let mut state = AppState::default();
    *state.login_flow_mut() = Some(crate::login_flow::LoginFlowState {
        step: crate::login_flow::LoginStep::KeyInput,
        provider: "anthropic".to_string(),
        key: "sk-test".to_string(),
        available_models: vec![],
        selected_models: std::collections::HashSet::new(),
        validated: false,
    });
    assert_eq!(state.active_provider(), "anthropic");
}

#[test]
fn active_provider_returns_config_default_when_no_flow() {
    let mut state = AppState::default();
    state.config_mut().current_provider = "openai".to_string();
    *state.login_flow_mut() = None;
    assert_eq!(state.active_provider(), "openai");
}

#[test]
fn active_provider_returns_config_default_when_no_flow_no_config() {
    // In test mode, default ConfigState sets current_provider to "mock"
    let mut state = AppState::default();
    *state.login_flow_mut() = None;
    // active_provider falls back to config.current_provider
    assert_eq!(state.active_provider(), "mock");
}
