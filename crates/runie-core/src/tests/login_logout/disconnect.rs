use crate::model::AppState;

use super::{add_minimax_provider, clean_config, select_minimax_model};

#[test]
fn providers_disconnect_removes_provider() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersDisconnect {
        provider: "minimax".into(),
    });

    assert!(
        state.config.current_provider != "minimax",
        "current provider should be cleared after disconnect"
    );
}

#[test]
fn providers_disconnect_opens_login_when_no_models_remain() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersDisconnect {
        provider: "minimax".into(),
    });

    assert!(
        state.login_flow.is_some(),
        "disconnecting the last provider should reopen the login flow"
    );
    assert!(
        !state.has_models(),
        "no model should be connected after disconnecting the last provider"
    );
}

#[test]
fn disconnect_clears_active_provider_when_no_other() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    assert_eq!(state.config.current_provider, "minimax");

    state.dialog_back_stack.clear();

    state.update(crate::Event::ProvidersDialog);
    state.update(crate::Event::ProvidersDisconnect {
        provider: "minimax".into(),
    });

    assert!(
        state.login_flow.is_some(),
        "disconnect should open login flow when no providers remain"
    );
}
