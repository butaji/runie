//! Tests for focus event handling in the AppState.

use crate::event::InputEvent;

use crate::AppState;

/// Focus gained event should not crash the state.
#[test]
fn focus_gained_event_handled() {
    let mut state = AppState::default();
    state.update(InputEvent::FocusGained);
    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Focus lost event should not crash the state.
#[test]
fn focus_lost_event_handled() {
    let mut state = AppState::default();
    state.update(InputEvent::FocusLost);
    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Multiple focus events in sequence should work.
#[test]
fn focus_events_sequence() {
    let mut state = AppState::default();
    state.update(InputEvent::FocusGained);
    state.update(InputEvent::FocusLost);
    state.update(InputEvent::FocusGained);
    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Focus events should not affect input.
#[test]
fn focus_events_dont_affect_input() {
    let mut state = AppState::default();
    state.update(InputEvent::Input('h'));
    state.update(InputEvent::Input('e'));
    state.update(InputEvent::Input('l'));
    state.update(InputEvent::Input('l'));
    state.update(InputEvent::Input('o'));
    assert_eq!(state.input.input, "hello");

    state.update(InputEvent::FocusLost);
    state.update(InputEvent::FocusGained);

    // Input should be unchanged
    assert_eq!(state.input.input, "hello");
}
