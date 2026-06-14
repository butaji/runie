use crate::commands::DialogState;
use crate::model::AppState;

use super::run_slash;

#[test]
fn hotkeys_opens_panel() {
    let mut state = AppState::default();
    run_slash(&mut state, "/hotkeys");
    assert!(
        matches!(state.open_dialog, Some(DialogState::PanelStack(_))),
        "/hotkeys should open a panel"
    );
}

#[test]
fn hotkeys_alias_keys_works() {
    let mut state = AppState::default();
    run_slash(&mut state, "/keys");
    assert!(
        matches!(state.open_dialog, Some(DialogState::PanelStack(_))),
        "/keys alias should open hotkeys panel"
    );
}
