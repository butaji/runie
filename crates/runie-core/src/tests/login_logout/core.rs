use crate::event::Event;
use crate::model::AppState;

use super::clean_config;

#[test]
fn providers_command_opens_dialog() {
    clean_config();
    let mut state = AppState::default();
    state.update(Event::ProvidersDialog);
    assert!(
        state.open_dialog.is_some(),
        "/providers must open the dialog"
    );
}

#[test]
fn slash_providers_opens_dialog() {
    clean_config();
    let mut state = AppState::default();

    for c in "/providers".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);

    assert!(
        state.open_dialog.is_some(),
        "raw /providers command should open the dialog"
    );
}

#[test]
fn slash_provider_alias_opens_dialog() {
    clean_config();
    let mut state = AppState::default();

    for c in "/provider".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);

    assert!(
        state.open_dialog.is_some(),
        "raw /provider command should open the dialog"
    );
}
