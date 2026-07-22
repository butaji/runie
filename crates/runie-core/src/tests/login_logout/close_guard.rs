use crate::model::AppState;

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
fn login_panel_quit_allowed_even_without_model() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);

    state.update(crate::Event::Quit);
    assert!(
        state.should_quit,
        "Quit must close the app even when onboarding is open"
    );
}

#[test]
fn login_panel_force_quit_allowed_without_model() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);

    state.update(crate::Event::ForceQuit);
    assert!(
        state.should_quit,
        "ForceQuit must quit the app even when no model is connected"
    );
}

#[test]
fn slash_quit_closes_even_during_onboarding() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);

    let result = state
        .handle_slash("/quit")
        .expect("/quit should be a command");
    state.apply_command_result(result);

    assert!(
        state.should_quit,
        "/quit must close the app even during onboarding"
    );
}

#[test]
fn login_panel_cancel_navigates_sub_panels_but_does_not_close() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider { provider: "minimax".into() });

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
fn login_panel_dialog_back_blocked_at_root_after_sub_panel_navigation() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider { provider: "minimax".into() });

    // Esc / DialogBack from the key input panel should pop back to the provider
    // picker, not close the whole onboarding dialog.
    state.update(crate::Event::DialogBack);

    let flow = state.login_flow.as_ref().unwrap();
    assert_eq!(
        flow.step,
        crate::login_flow::LoginStep::ProviderPicker,
        "DialogBack should return to the provider picker"
    );
    assert!(
        state.open_dialog.is_some(),
        "onboarding dialog should still be open"
    );

    let stack = state.open_dialog.as_ref().unwrap().panel_stack().unwrap();
    assert!(
        !stack.root().unwrap().closable,
        "root panel must stay non-closable while no model is connected"
    );

    // A second DialogBack at the root must still be blocked.
    state.update(crate::Event::DialogBack);
    assert!(
        state.open_dialog.is_some(),
        "DialogBack at root should not close onboarding without a connected model"
    );
}

#[test]
fn login_panel_close_allowed_once_model_connected() {
    clean_config();
    let mut state = disconnected_state();
    state.update(crate::Event::Start);
    state.update(crate::Event::SelectProvider { provider: "minimax".into() });
    state.update(crate::Event::SubmitKey { provider: "minimax".into(), key: "sk-test".into() });
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

struct MockGuard(bool);

impl MockGuard {
    fn enabled() -> Self {
        crate::provider::set_mock_enabled(true);
        Self(true)
    }
}

impl Drop for MockGuard {
    fn drop(&mut self) {
        if self.0 {
            crate::provider::set_mock_enabled(false);
        }
    }
}

#[test]
fn login_panel_dialog_back_blocked_when_only_mock_fallback() {
    clean_config();
    let _guard = MockGuard::enabled();

    // In test mode AppState defaults to the mock provider/model, so the user
    // appears "connected" but only via the RUNIE_MOCK fallback.
    let mut state = AppState::default();
    state.update(crate::Event::Start);

    assert!(state.login_flow.is_some(), "login flow should be open");
    assert!(state.has_models(), "mock fallback should look connected");

    state.update(crate::Event::DialogBack);

    assert!(
        state.login_flow.is_some(),
        "DialogBack should not close onboarding when only the mock fallback is active"
    );
    assert!(state.open_dialog.is_some(), "login panel should stay open");
}
