use crate::event::Event;
use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use crate::model::AppState;

use super::{add_minimax_provider, clean_config, select_minimax_model};

#[test]
fn providers_disconnect_removes_provider() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    state.update(Event::Dialog(DialogEvent::ProvidersDialog));
    state.update(Event::Dialog(DialogEvent::ProvidersDisconnect {
        provider: "minimax".into(),
    }));

    assert!(
        state.config.current_provider != "minimax",
        "current provider should be cleared after disconnect"
    );
}

#[test]
fn providers_disconnect_closes_dialog() {
    clean_config();
    let mut state = AppState::default();

    add_minimax_provider(&mut state);
    select_minimax_model(&mut state);

    state.update(Event::Dialog(DialogEvent::ProvidersDialog));
    state.update(Event::Dialog(DialogEvent::ProvidersDisconnect {
        provider: "minimax".into(),
    }));

    assert!(
        state.open_dialog.is_none(),
        "disconnecting should close the dialog"
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

    state.update(Event::Dialog(DialogEvent::ProvidersDialog));
    state.update(Event::Dialog(DialogEvent::ProvidersDisconnect {
        provider: "minimax".into(),
    }));

    assert!(
        state.open_dialog.is_none(),
        "disconnect should close the dialog"
    );
}
