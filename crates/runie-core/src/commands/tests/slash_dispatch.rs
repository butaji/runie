use crate::config::ModelProvider;
use crate::model::AppState;

use super::run_slash;

fn seed_provider(state: &mut AppState, name: &str, models: Vec<String>) {
    state.config_mut().model_providers_mut().insert(
        name.into(),
        ModelProvider {
            provider_type: None,
            base_url: String::new(),
            models,
            headers: std::collections::HashMap::new(),
            context_window_fallbacks: vec![],
        },
    );
}

#[test]
fn slash_event_dispatches_to_registry() {
    let mut state = AppState::default();
    seed_provider(&mut state, "mock", vec!["gpt-4o".into()]);
    run_slash(&mut state, "/model gpt-4o");
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn alias_event_dispatches_correctly() {
    let mut state = AppState::default();
    seed_provider(&mut state, "mock", vec!["gpt-4o".into()]);
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

#[test]
fn exit_alias_quits_immediately() {
    let mut state = AppState::default();
    run_slash(&mut state, "/exit");
    assert!(
        state.should_quit,
        "/exit should resolve to /quit and set should_quit"
    );
}

#[test]
fn q_alias_quits_immediately() {
    let mut state = AppState::default();
    run_slash(&mut state, "/q");
    assert!(
        state.should_quit,
        "/q should resolve to /quit and set should_quit"
    );
}

#[test]
fn colon_q_quits_immediately() {
    let mut state = AppState::default();
    run_slash(&mut state, ":q");
    assert!(
        state.should_quit,
        ":q should quit immediately without slash prefix"
    );
}
