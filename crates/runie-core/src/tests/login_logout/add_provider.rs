use crate::event::Event;
use crate::model::AppState;

use super::clean_config;

#[test]
fn providers_add_starts_login_flow() {
    clean_config();
    let mut state = AppState::default();
    state.update(Event::ProvidersDialog);
    assert!(
        state.open_dialog.is_some(),
        "providers dialog should be open"
    );

    state.update(Event::ProvidersAdd);

    assert!(state.login_flow.is_some(), "login flow should start");
    assert!(
        !state.dialog_back_stack.is_empty(),
        "providers dialog should be on back stack"
    );
}

#[test]
fn login_flow_cancel_returns_to_providers_dialog() {
    clean_config();
    let mut state = AppState::default();

    state.update(Event::ProvidersDialog);
    assert!(state.open_dialog.is_some());

    state.update(Event::ProvidersAdd);
    assert!(state.login_flow.is_some());

    state.update(Event::LoginFlowCancel);

    assert!(
        state.login_flow.is_none(),
        "login flow should be cleared on cancel"
    );

    let restored = state.open_dialog.is_some() || !state.dialog_back_stack.is_empty();
    assert!(restored, "cancel should return to previous dialog");
}
