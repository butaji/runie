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

/// Layer 1: round-trip via crokey parse → KeyCombination → to_string → lowercase.
/// crokey uses `-` as the separator between modifiers and keys.
#[test]
fn key_combo_round_trip_ctrl_c() {
    let combo = crokey::parse("ctrl-c").unwrap();
    let result = crate::keymap::key_event_to_combo(&combo.into());
    assert_eq!(result, "ctrl+c");
}

#[test]
fn key_combo_round_trip_alt_enter() {
    let combo = crokey::parse("alt-enter").unwrap();
    let result = crate::keymap::key_event_to_combo(&combo.into());
    assert_eq!(result, "alt+enter");
}

#[test]
fn key_combo_round_trip_shift_tab() {
    // BackTab is aliased to "shift+tab" in the binding table (legacy convention).
    let combo = crokey::parse("shift-backtab").unwrap();
    let result = crate::keymap::key_event_to_combo(&combo.into());
    assert_eq!(result, "shift+tab");
}

#[test]
fn key_combo_round_trip_uppercase_modifiers() {
    // crokey is case-insensitive on parse; output should be lowercase.
    let combo = crokey::parse("Ctrl-Shift-M").unwrap();
    let result = crate::keymap::key_event_to_combo(&combo.into());
    assert_eq!(result, "ctrl+shift+m");
}
