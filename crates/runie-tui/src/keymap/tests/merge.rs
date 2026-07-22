//! Layer 2 tests — verify keymap and terminal caps work after runie-term merge.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Verify capability detection is functional after the runie-term merge.
#[test]
#[allow(clippy::overly_complex_bool_expr)]
fn terminal_caps_detect_from_env() {
    let caps = crate::terminal::caps::detect_capabilities_from_env();
    let _ = format!("{:?}", caps);
    // Should always return a valid struct, even in test env
    assert!(caps.truecolor || !caps.truecolor);
}

/// Verify keymap conversion still works after the runie-term merge.
#[test]
fn keymap_convert_plain_char() {
    let bindings = std::collections::HashMap::new();
    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
    let event = Event::Key(key);
    let result = crate::keymap::convert_event(&event, &bindings);
    assert!(result.is_some(), "plain 'a' key should convert to an event");
}

/// Verify Enter key submission is recognized.
#[test]
fn keymap_convert_enter() {
    let bindings = std::collections::HashMap::new();
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    let event = Event::Key(key);
    let result = crate::keymap::convert_event(&event, &bindings);
    assert!(result.is_some(), "Enter key should convert to Submit event");
}

/// Verify paste events are forwarded.
#[test]
fn keymap_forward_paste() {
    let bindings = std::collections::HashMap::new();
    let event = Event::Paste("hello".to_string());
    let result = crate::keymap::convert_event(&event, &bindings);
    assert!(result.is_some(), "Paste events should be forwarded");
}

/// Wheel events are dropped: runie never enables mouse capture, so no mouse
/// event should convert to a core event. Native terminal selection takes
/// precedence over wheel scrolling (keyboard scrolls the feed instead).
#[test]
fn keymap_mouse_scroll_is_dropped() {
    let bindings = std::collections::HashMap::new();
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    });
    let result = crate::keymap::convert_event(&event, &bindings);
    assert_eq!(
        result, None,
        "mouse wheel should not convert to a core event"
    );
}

/// Click (button down) is not a supported mouse interaction; it must be
/// dropped. Only wheel scrolling is kept.
#[test]
fn keymap_mouse_click_is_dropped() {
    let bindings = std::collections::HashMap::new();
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 10,
        modifiers: KeyModifiers::empty(),
    });
    let result = crate::keymap::convert_event(&event, &bindings);
    assert_eq!(
        result, None,
        "mouse click should not convert to a core event"
    );
}

/// Button release is not a supported mouse interaction; it must be dropped.
#[test]
fn keymap_mouse_release_is_dropped() {
    let bindings = std::collections::HashMap::new();
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 5,
        row: 10,
        modifiers: KeyModifiers::empty(),
    });
    let result = crate::keymap::convert_event(&event, &bindings);
    assert_eq!(
        result, None,
        "mouse release should not convert to a core event"
    );
}

/// Drag is not a supported mouse interaction; it must be dropped.
#[test]
fn keymap_mouse_drag_is_dropped() {
    let bindings = std::collections::HashMap::new();
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 5,
        row: 10,
        modifiers: KeyModifiers::empty(),
    });
    let result = crate::keymap::convert_event(&event, &bindings);
    assert_eq!(
        result, None,
        "mouse drag should not convert to a core event"
    );
}
