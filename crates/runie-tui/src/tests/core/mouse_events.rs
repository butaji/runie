//! Tests for the `MouseScrollUp`/`MouseScrollDown` core events in AppState.
//!
//! These events remain in the core event taxonomy, but runie no longer
//! enables terminal mouse capture, so terminal input never produces them
//! (native selection owns the mouse). They stay handled so replayed event
//! logs keep working.

use runie_core::AppState;
use runie_core::Event;

/// Mouse wheel scroll up should scroll without panic.
#[test]
fn mouse_scroll_up_works() {
    let mut state = AppState::default();

    state.update(Event::MouseScrollUp);
    // Should not panic
    let _ = state.view.scroll;
}

/// Mouse wheel scroll down should scroll without panic.
#[test]
fn mouse_scroll_down_works() {
    let mut state = AppState::default();

    state.update(Event::MouseScrollDown);
    // Should not panic
    let _ = state.view.scroll;
}
