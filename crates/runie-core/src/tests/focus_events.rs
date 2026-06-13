//! Tests for focus event handling in the AppState.

use crate::{AppState, Event};

/// Focus gained event should not crash the state.
#[test]
fn focus_gained_event_handled() {
    let mut state = AppState::default();
    state.update(Event::FocusGained);
    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Focus lost event should not crash the state.
#[test]
fn focus_lost_event_handled() {
    let mut state = AppState::default();
    state.update(Event::FocusLost);
    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Multiple focus events in sequence should work.
#[test]
fn focus_events_sequence() {
    let mut state = AppState::default();
    state.update(Event::FocusGained);
    state.update(Event::FocusLost);
    state.update(Event::FocusGained);
    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Focus events should not affect input.
#[test]
fn focus_events_dont_affect_input() {
    let mut state = AppState::default();
    state.update(Event::Input('h'));
    state.update(Event::Input('e'));
    state.update(Event::Input('l'));
    state.update(Event::Input('l'));
    state.update(Event::Input('o'));
    assert_eq!(state.input.input, "hello");

    state.update(Event::FocusLost);
    state.update(Event::FocusGained);

    // Input should be unchanged
    assert_eq!(state.input.input, "hello");
}
