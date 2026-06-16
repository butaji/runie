use crate::event::{InputEvent, DialogEvent};
use crate::model::AppState;

use super::clean_config;

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(InputEvent::Input('/'));
    for c in cmd.chars() {
        state.update(DialogEvent::PaletteFilter(c));
    }
    state.update(DialogEvent::PaletteSelect);
}

#[test]
fn providers_command_opens_dialog() {
    clean_config();
    let mut state = AppState::default();
    state.update(DialogEvent::ProvidersDialog);
    assert!(
        state.open_dialog.is_some(),
        "/providers must open the dialog"
    );
}

#[test]
fn slash_providers_opens_dialog() {
    clean_config();
    let mut state = AppState::default();
    palette_select(&mut state, "providers");

    assert!(
        state.open_dialog.is_some(),
        "raw /providers command should open the dialog"
    );
}

#[test]
fn slash_provider_alias_opens_dialog() {
    clean_config();
    let mut state = AppState::default();
    palette_select(&mut state, "provider");

    assert!(
        state.open_dialog.is_some(),
        "raw /provider command should open the dialog"
    );
}
