use crate::event::{DialogEvent, LoginFlowEvent};
use crate::model::AppState;

use super::{clean_config, validate_provider};

#[test]
fn providers_add_starts_login_flow() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(DialogEvent::ProvidersDialog);
    assert!(
        state.open_dialog.is_some(),
        "providers dialog should be open"
    );

    state.update(DialogEvent::ProvidersAdd);

    assert!(state.login_flow.is_some(), "login flow should start");
    assert!(
        !state.dialog_back_stack.is_empty(),
        "providers dialog should be on back stack"
    );
}

#[test]
fn login_flow_cancel_blocked_without_model() {
    clean_config();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersAdd);
    assert!(state.login_flow.is_some());

    state.update(LoginFlowEvent::Cancel);

    assert!(
        state.login_flow.is_some(),
        "cancel should be blocked when no model is connected"
    );
    assert!(state.open_dialog.is_some(), "login panel should stay open");
}

#[test]
fn login_flow_cancel_allowed_with_model() {
    clean_config();
    let mut state = AppState::default();

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersAdd);
    state.update(LoginFlowEvent::SelectProvider {
        provider: "minimax".into(),
    });
    state.update(LoginFlowEvent::SubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    validate_provider(&mut state, "minimax", "sk-test");
    state.update(LoginFlowEvent::Save);

    assert!(state.login_flow.is_none());
    assert!(state.has_models());

    state.update(DialogEvent::ProvidersAdd);
    assert!(state.login_flow.is_some());

    state.update(LoginFlowEvent::Cancel);

    assert!(
        state.login_flow.is_none(),
        "cancel should close login flow when a model is already connected"
    );
}
