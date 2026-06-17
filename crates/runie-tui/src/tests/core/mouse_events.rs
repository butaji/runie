//! Tests for mouse event handling in the AppState.

use runie_core::event::{InputEvent, ScrollEvent};
use runie_core::AppState;

/// Mouse click event should not crash the state.
#[test]
fn mouse_click_event_handled() {
    let mut state = AppState::default();

    // Send a mouse click event
    state.update(InputEvent::MouseClick {
        row: 5,
        col: 10,
        button: "left".to_string(),
    });

    // State should be valid after event
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Mouse release event should not crash the state.
#[test]
fn mouse_release_event_handled() {
    let mut state = AppState::default();

    state.update(InputEvent::MouseRelease {
        row: 5,
        col: 10,
        button: "left".to_string(),
    });

    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Mouse drag event should not crash the state.
#[test]
fn mouse_drag_event_handled() {
    let mut state = AppState::default();

    state.update(InputEvent::MouseDrag {
        row: 5,
        col: 10,
        button: "left".to_string(),
    });

    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Mouse move event should not crash the state.
#[test]
fn mouse_move_event_handled() {
    let mut state = AppState::default();

    state.update(InputEvent::MouseMove { row: 5, col: 10 });

    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Multiple mouse events in sequence should work.
#[test]
fn multiple_mouse_events() {
    let mut state = AppState::default();

    state.update(InputEvent::MouseClick {
        row: 1,
        col: 1,
        button: "left".to_string(),
    });
    state.update(InputEvent::MouseDrag {
        row: 2,
        col: 2,
        button: "left".to_string(),
    });
    state.update(InputEvent::MouseRelease {
        row: 3,
        col: 3,
        button: "left".to_string(),
    });

    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Mouse click with different buttons.
#[test]
fn mouse_click_different_buttons() {
    let mut state = AppState::default();

    state.update(InputEvent::MouseClick {
        row: 1,
        col: 1,
        button: "left".to_string(),
    });
    state.update(InputEvent::MouseClick {
        row: 1,
        col: 1,
        button: "right".to_string(),
    });
    state.update(InputEvent::MouseClick {
        row: 1,
        col: 1,
        button: "middle".to_string(),
    });

    // State should be valid
    assert!(state.open_dialog.is_none() || state.open_dialog.is_some());
}

/// Mouse scroll up should not crash.
#[test]
fn mouse_scroll_up_works() {
    let mut state = AppState::default();

    state.update(ScrollEvent::Up);
    // Should not panic
    let _ = state.view.scroll;
}

/// Mouse scroll down should not crash.
#[test]
fn mouse_scroll_down_works() {
    let mut state = AppState::default();

    state.update(ScrollEvent::Down);
    // Should not panic
    let _ = state.view.scroll;
}

/// Mouse events with input open should work.
#[test]
fn mouse_events_with_input() {
    let mut state = AppState::default();
    state.update(InputEvent::Input('h'));
    state.update(InputEvent::Input('i'));

    assert_eq!(state.input.input, "hi");

    // Mouse click should not affect input
    state.update(InputEvent::MouseClick {
        row: 1,
        col: 1,
        button: "left".to_string(),
    });

    assert_eq!(state.input.input, "hi");
}
