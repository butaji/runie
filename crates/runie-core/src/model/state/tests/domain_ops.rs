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

#[test]
fn apply_config_populates_mode_section() {
    let mut state = AppState::default();
    assert_eq!(state.config().mode, crate::config::ModeSection::default());

    let config = crate::config::Config {
        mode: crate::config::ModeSection {
            active: "swarm".into(),
            workers: 6,
            ..crate::config::ModeSection::default()
        },
        ..crate::config::Config::default()
    };
    state.apply_config(&config);
    assert_eq!(state.config().mode.active, "swarm");
    assert_eq!(state.config().mode.workers, 6);
    assert_eq!(state.config().mode.max_rounds, 5);
}
