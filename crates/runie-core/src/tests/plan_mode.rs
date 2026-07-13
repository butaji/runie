//! Plan mode input handling: Esc must disable plan mode.
//!
//! The plan panel advertises `[Enter] Approve plan   [Esc] /plan off`, but a
//! plain Esc keypress maps to `DialogBack` (dialog-category) since the keymap
//! refactor, while plan mode is a view flag handled by the input router — so
//! Esc fell through and plan mode stayed on. `Event::Escape` (test/legacy
//! path) must behave the same.

use crate::model::AppState;

#[test]
fn dialog_back_disables_plan_mode() {
    let mut state = AppState::default();
    state.view_mut().plan_mode = true;
    state.view_mut().active_plan_content = "1. do the thing".into();

    // Plain Esc maps to DialogBack since the keymap refactor.
    state.update(crate::Event::DialogBack);

    assert!(
        !state.view().plan_mode,
        "Esc (DialogBack) must disable plan mode"
    );
    assert!(
        state.view().active_plan_content.is_empty(),
        "plan content must be cleared on Esc"
    );
}

#[test]
fn escape_event_disables_plan_mode() {
    let mut state = AppState::default();
    state.view_mut().plan_mode = true;
    state.view_mut().active_plan_content = "1. do the thing".into();

    state.update(crate::Event::Escape);

    assert!(
        !state.view().plan_mode,
        "Event::Escape must disable plan mode"
    );
    assert!(
        state.view().active_plan_content.is_empty(),
        "plan content must be cleared on Escape"
    );
}

#[test]
fn enter_still_approves_plan_mode() {
    let mut state = AppState::default();
    state.view_mut().plan_mode = true;
    state.view_mut().active_plan_content = "1. do the thing".into();

    state.update(crate::Event::Submit);

    assert!(
        !state.view().plan_mode,
        "Enter must approve (disable) plan mode"
    );
}
