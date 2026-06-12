//! Tests for keyboard handlers - specifically q/Esc key handling in shortcuts panel.

use crossterm::event::{KeyCode, KeyEvent};
use crate::tui::state::Msg;
use super::handlers::{
    shortcuts_panel_normal_msg,
    shortcuts_panel_filter_msg,
    key_to_shortcuts_panel_msg,
};

#[test]
fn test_q_closes_shortcuts_panel_via_normal_msg() {
    let key = KeyEvent::from(KeyCode::Char('q'));
    let msg = shortcuts_panel_normal_msg(key);
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_esc_closes_shortcuts_panel_via_normal_msg() {
    let key = KeyEvent::from(KeyCode::Esc);
    let msg = shortcuts_panel_normal_msg(key);
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_q_does_not_return_textarea_key_in_normal_mode() {
    // When shortcuts panel is open and in normal mode, 'q' should close the panel
    // and NOT be passed to the textarea as a character input
    let key = KeyEvent::from(KeyCode::Char('q'));
    let msg = shortcuts_panel_normal_msg(key);
    // Must NOT be TextareaKey
    assert!(!matches!(msg, Some(Msg::TextareaKey(_))));
    // Must be CloseShortcutsPanel
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_other_char_keys_do_not_close_shortcuts_panel() {
    // Test that keys like 'a', 'b', 'c', etc. do NOT close the shortcuts panel
    let keys_to_test = ['a', 'b', 'c', 'w', 'e', 'f', 'r', 't', 'y', 'u', 'i', 'o', 'p'];
    for ch in keys_to_test {
        let key = KeyEvent::from(KeyCode::Char(ch));
        let msg = shortcuts_panel_normal_msg(key);
        assert!(
            !matches!(msg, Some(Msg::CloseShortcutsPanel)),
            "Char '{}' should NOT close shortcuts panel, but got {:?}",
            ch, msg
        );
    }
}

#[test]
fn test_navigation_keys_do_not_close_shortcuts_panel() {
    // Up/Down should navigate, not close
    let up_key = KeyEvent::from(KeyCode::Up);
    let down_key = KeyEvent::from(KeyCode::Down);
    assert_eq!(shortcuts_panel_normal_msg(up_key), Some(Msg::ShortcutsPanelUp));
    assert_eq!(shortcuts_panel_normal_msg(down_key), Some(Msg::ShortcutsPanelDown));
}

#[test]
fn test_filter_toggle_keys_do_not_close_shortcuts_panel() {
    // 'f' and '/' toggle filter, they don't close the panel
    let f_key = KeyEvent::from(KeyCode::Char('f'));
    let slash_key = KeyEvent::from(KeyCode::Char('/'));
    assert_eq!(shortcuts_panel_normal_msg(f_key), Some(Msg::ShortcutsPanelToggleFilter));
    assert_eq!(shortcuts_panel_normal_msg(slash_key), Some(Msg::ShortcutsPanelToggleFilter));
}

#[test]
fn test_section_toggle_keys_do_not_close_shortcuts_panel() {
    // 'e', Enter, Space toggle section, they don't close the panel
    let e_key = KeyEvent::from(KeyCode::Char('e'));
    let enter_key = KeyEvent::from(KeyCode::Enter);
    let space_key = KeyEvent::from(KeyCode::Char(' '));
    assert_eq!(shortcuts_panel_normal_msg(e_key), Some(Msg::ShortcutsPanelToggleSection));
    assert_eq!(shortcuts_panel_normal_msg(enter_key), Some(Msg::ShortcutsPanelToggleSection));
    assert_eq!(shortcuts_panel_normal_msg(space_key), Some(Msg::ShortcutsPanelToggleSection));
}

// Filter mode tests

#[test]
fn test_q_closes_shortcuts_panel_via_filter_msg() {
    let key = KeyEvent::from(KeyCode::Char('q'));
    let msg = shortcuts_panel_filter_msg(key);
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_esc_closes_shortcuts_panel_via_filter_msg() {
    let key = KeyEvent::from(KeyCode::Esc);
    let msg = shortcuts_panel_filter_msg(key);
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_q_does_not_return_textarea_key_in_filter_mode() {
    let key = KeyEvent::from(KeyCode::Char('q'));
    let msg = shortcuts_panel_filter_msg(key);
    assert!(!matches!(msg, Some(Msg::TextareaKey(_))));
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_filter_input_char_does_not_close_panel() {
    // In filter mode, chars are used for filtering
    let key = KeyEvent::from(KeyCode::Char('a'));
    let msg = shortcuts_panel_filter_msg(key);
    assert_eq!(msg, Some(Msg::ShortcutsPanelFilterInput('a')));
    assert!(!matches!(msg, Some(Msg::CloseShortcutsPanel)));
}

#[test]
fn test_filter_backspace_does_not_close_panel() {
    let key = KeyEvent::from(KeyCode::Backspace);
    let msg = shortcuts_panel_filter_msg(key);
    assert_eq!(msg, Some(Msg::ShortcutsPanelFilterBackspace));
    assert!(!matches!(msg, Some(Msg::CloseShortcutsPanel)));
}

// Integration test: key_to_shortcuts_panel_msg with state

fn create_shortcuts_panel_state(filter_mode: bool) -> crate::tui::state::AppState {
    use crate::components::ShortcutsPanel;
    let panel = ShortcutsPanel {
        filter_mode,
        ..Default::default()
    };
    crate::tui::state::AppState {
        shortcuts_panel: panel,
        ..Default::default()
    }
}

#[test]
fn test_key_to_shortcuts_panel_msg_normal_mode_q_closes() {
    let state = create_shortcuts_panel_state(false);
    let key = KeyEvent::from(KeyCode::Char('q'));
    let msg = key_to_shortcuts_panel_msg(key, &state);
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_key_to_shortcuts_panel_msg_filter_mode_q_closes() {
    let state = create_shortcuts_panel_state(true);
    let key = KeyEvent::from(KeyCode::Char('q'));
    let msg = key_to_shortcuts_panel_msg(key, &state);
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_key_to_shortcuts_panel_msg_normal_mode_esc_closes() {
    let state = create_shortcuts_panel_state(false);
    let key = KeyEvent::from(KeyCode::Esc);
    let msg = key_to_shortcuts_panel_msg(key, &state);
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_key_to_shortcuts_panel_msg_filter_mode_esc_closes() {
    let state = create_shortcuts_panel_state(true);
    let key = KeyEvent::from(KeyCode::Esc);
    let msg = key_to_shortcuts_panel_msg(key, &state);
    assert_eq!(msg, Some(Msg::CloseShortcutsPanel));
}

#[test]
fn test_key_to_shortcuts_panel_msg_filter_mode_char_filters() {
    let state = create_shortcuts_panel_state(true);
    let key = KeyEvent::from(KeyCode::Char('t'));
    let msg = key_to_shortcuts_panel_msg(key, &state);
    assert_eq!(msg, Some(Msg::ShortcutsPanelFilterInput('t')));
}

#[test]
fn test_key_to_shortcuts_panel_msg_normal_mode_char_does_not_close() {
    let state = create_shortcuts_panel_state(false);
    // 'w' in normal mode doesn't close panel (it's not a bound action)
    let key = KeyEvent::from(KeyCode::Char('w'));
    let msg = key_to_shortcuts_panel_msg(key, &state);
    // Should return None (unhandled), not CloseShortcutsPanel
    assert_eq!(msg, None);
}
