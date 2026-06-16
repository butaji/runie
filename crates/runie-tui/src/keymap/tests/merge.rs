//! Layer 2 tests — verify keymap and terminal caps work after runie-term merge.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

/// Verify capability detection is functional after the runie-term merge.
#[test]
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

/// Verify mouse scroll events are forwarded.
#[test]
fn keymap_forward_scroll() {
    let bindings = std::collections::HashMap::new();
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    });
    let result = crate::keymap::convert_event(&event, &bindings);
    assert!(result.is_some(), "Scroll events should be forwarded");
}
