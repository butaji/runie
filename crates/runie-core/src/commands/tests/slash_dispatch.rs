use crate::model::AppState;

use super::run_slash;

#[test]
fn slash_event_dispatches_to_registry() {
    crate::login_config::set_test_config_with_providers(&[("mock".into(), vec!["gpt-4o".into()])]);
    let mut state = AppState::default();
    state.populate_cache_from_login_config();
    run_slash(&mut state, "/model gpt-4o");
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn alias_event_dispatches_correctly() {
    crate::login_config::set_test_config_with_providers(&[("mock".into(), vec!["gpt-4o".into()])]);
    let mut state = AppState::default();
    state.populate_cache_from_login_config();
    run_slash(&mut state, "/m gpt-4o");
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn provider_alias_dispatches_to_same_command() {
    let mut state = AppState::default();
    run_slash(&mut state, "/provider");
    assert!(
        state.open_dialog.is_some(),
        "/provider should open providers dialog"
    );

    let mut state = AppState::default();
    run_slash(&mut state, "/providers");
    assert!(
        state.open_dialog.is_some(),
        "/providers should open providers dialog"
    );
}
