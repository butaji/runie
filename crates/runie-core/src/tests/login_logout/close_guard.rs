use crate::model::AppState;
use crate::Event;

use super::{clean_config, validate_provider};

fn disconnected_state() -> AppState {
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state
}

#[test]
fn login_panel_abort_blocked_without_model() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);
    assert!(state.login_flow.is_some());

    state.update(crate::Event::Abort);

    assert!(
        state.login_flow.is_some(),
        "Abort should not close the login panel when no model is connected"
    );
    assert!(state.open_dialog.is_some(), "login panel should stay open");
}

#[test]
fn login_panel_dialog_back_blocked_without_model() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);
    assert!(state.login_flow.is_some());

    state.update(crate::Event::DialogBack);

    assert!(
        state.login_flow.is_some(),
        "DialogBack should not close the login panel when no model is connected"
    );
    assert!(state.open_dialog.is_some(), "login panel should stay open");
}

#[test]
fn login_panel_quit_blocked_but_force_quit_allowed() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);

    state.update(crate::Event::Quit);
    assert!(
        !state.should_quit,
        "Quit should be blocked when no model is connected"
    );

    state.update(crate::Event::ForceQuit);
    assert!(
        state.should_quit,
        "ForceQuit must quit the app even when no model is connected"
    );
}

#[test]
fn login_panel_cancel_navigates_sub_panels_but_does_not_close() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(flow.step, crate::login_flow::LoginStep::KeyInput);

    state.update(crate::Event::Abort);

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(
        flow.step,
        crate::login_flow::LoginStep::ProviderPicker,
        "Cancel should pop back to provider picker"
    );
    assert!(
        state.open_dialog.is_some(),
        "login panel should stay open after popping a sub-panel"
    );
}

#[test]
fn login_panel_close_allowed_once_model_connected() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(crate::Event::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");
    state.update(crate::Event::Save);

    assert!(state.has_models());

    state.update(crate::Event::Start);
    state.update(crate::Event::Abort);

    assert!(
        state.login_flow.is_none(),
        "Abort should close login panel once a model is connected"
    );
}
