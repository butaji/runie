use super::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn key_event_to_combo_ctrl_c() {
    let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(crate::keymap::key_event_to_combo(&key), "ctrl+c");
}

#[test]
fn key_event_to_combo_alt_enter() {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT);
    assert_eq!(crate::keymap::key_event_to_combo(&key), "alt+enter");
}

#[test]
fn key_event_to_combo_shift_enter() {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
    assert_eq!(crate::keymap::key_event_to_combo(&key), "shift+enter");
}

#[test]
fn key_event_to_combo_plain_escape() {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    assert_eq!(crate::keymap::key_event_to_combo(&key), "escape");
}
