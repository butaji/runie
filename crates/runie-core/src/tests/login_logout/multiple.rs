use crate::event::DialogEvent;
use crate::model::AppState;

use super::{add_provider_and_select_model, clean_config};

#[test]
fn disconnect_active_provider_switches_to_another() {
    clean_config();
    let mut state = AppState::default();

    add_provider_and_select_model(&mut state, "minimax", "sk-test", "MiniMax-M3");
    add_provider_and_select_model(&mut state, "openai", "sk-test-openai", "gpt-4o");

    assert_eq!(state.config.current_provider, "openai");

    state.dialog_back_stack.clear();

    state.update(DialogEvent::ProvidersDialog);
    state.update(DialogEvent::ProvidersDisconnect {
        provider: "openai".into(),
    });

    assert_ne!(
        state.config.current_provider, "openai",
        "openai should not be current after disconnect"
    );
    assert!(
        state.open_dialog.is_none(),
        "dialog should be closed after disconnect"
    );
}
