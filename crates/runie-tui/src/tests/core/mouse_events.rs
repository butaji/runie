//! Tests for mouse wheel scrolling in the AppState.
//!
//! The only mouse interaction runie keeps is wheel scroll up/down in the feed.
//! Click, release, drag, and move are not handled.

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
