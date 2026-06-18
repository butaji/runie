//! Onboarding happy path tests.

use crate::event::{ControlEvent, LoginFlowEvent};
use crate::login_flow::LoginStep;
use crate::model::AppState;

use super::{
    assert_panel_id, assert_step, clean_config, fetch_models, save_login_flow, select_provider,
    start_login_flow, submit_key,
};

#[test]
fn auto_open_starts_provider_picker_when_no_provider() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);

    assert_step(&state, LoginStep::ProviderPicker);
    assert_panel_id(&state, "login-provider");
}

#[test]
fn full_happy_path_connects_provider() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".into()]);
    save_login_flow(&mut state);

    assert!(
        state.has_models(),
        "provider should be connected after save"
    );
    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(state.config.current_model, "MiniMax-M3");
    assert!(state.login_flow.is_none(), "login flow should be cleared");
    assert!(state.open_dialog.is_none(), "dialog should be closed");
}

#[test]
fn save_activates_first_selected_model() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    fetch_models(&mut state, &["MiniMax-M3".into(), "MiniMax-M2".into()]);
    save_login_flow(&mut state);

    assert_eq!(state.config.current_provider, "minimax");
    assert_eq!(
        state.config.current_model, "MiniMax-M3",
        "first model in available_models order should be activated"
    );
}

#[test]
fn validation_done_legacy_reaches_model_select() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    start_login_flow(&mut state);
    select_provider(&mut state, "minimax");
    submit_key(&mut state, "sk-test");
    state.update(LoginFlowEvent::ValidationDone {
        provider: "minimax".into(),
        key: "sk-test".into(),
        models: vec!["MiniMax-M3".into()],
    });

    assert_step(&state, LoginStep::ModelSelect);
}

#[test]
fn start_with_existing_model_does_not_auto_open() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider = "mock".into();
    state.config.current_model = "echo".into();

    assert!(state.has_models(), "test setup should have a model");
    assert!(
        state.login_flow.is_none(),
        "login flow should not open automatically when a model is connected"
    );

    // An explicit Start still opens the provider picker, but it is closable.
    start_login_flow(&mut state);
    assert_step(&state, LoginStep::ProviderPicker);
    assert_panel_id(&state, "login-provider");

    state.update(ControlEvent::Abort);
    assert!(
        state.login_flow.is_none(),
        "picker should close because a model is already connected"
    );
}
