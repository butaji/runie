//! Hotkey regression tests - Verify key events produce correct Msgs.

use crossterm::event::{KeyCode, KeyModifiers};
use crate::tui::state::{TuiMode, Msg};
use super::helpers::simulate_key;

#[test]
fn test_all_ctrl_keys_in_chat_mode() {
    let test_cases = vec![
        (KeyCode::Char('c'), KeyModifiers::CONTROL, Msg::Quit, "Ctrl+C"),
        (KeyCode::Char('q'), KeyModifiers::CONTROL, Msg::Quit, "Ctrl+Q"),
        (KeyCode::Char('j'), KeyModifiers::CONTROL, Msg::InsertNewline, "Ctrl+J"),
        (KeyCode::Char('k'), KeyModifiers::CONTROL, Msg::OpenCommandPalette, "Ctrl+K"),
        (KeyCode::Char('p'), KeyModifiers::CONTROL, Msg::OpenCommandPalette, "Ctrl+P"),
        (KeyCode::Char('a'), KeyModifiers::CONTROL, Msg::MoveCursorToStart, "Ctrl+A"),
        (KeyCode::Char('e'), KeyModifiers::CONTROL, Msg::MoveCursorToEnd, "Ctrl+E"),
        (KeyCode::Char('w'), KeyModifiers::CONTROL, Msg::DeleteWordBackward, "Ctrl+W"),
        (KeyCode::Char('u'), KeyModifiers::CONTROL, Msg::DeleteToStart, "Ctrl+U"),
        (KeyCode::Char('d'), KeyModifiers::CONTROL, Msg::DeleteForward, "Ctrl+D"),
        (KeyCode::Char('b'), KeyModifiers::CONTROL, Msg::ToggleSidebar, "Ctrl+B"),
        (KeyCode::Char('f'), KeyModifiers::CONTROL, Msg::MoveCursorRight, "Ctrl+F"),
        (KeyCode::Char('n'), KeyModifiers::CONTROL, Msg::MoveCursorDown, "Ctrl+N"),
        (KeyCode::Char('h'), KeyModifiers::CONTROL, Msg::Backspace, "Ctrl+H"),
    ];

    for (code, modifiers, expected_msg, name) in test_cases {
        let msg = simulate_key(code, modifiers, TuiMode::Chat);
        assert_eq!(msg, Some(expected_msg), "{} should produce correct Msg", name);
    }
}

#[test]
fn test_nav_keys_in_chat_mode() {
    let test_cases = vec![
        (KeyCode::Left, Msg::MoveCursorLeft),
        (KeyCode::Right, Msg::MoveCursorRight),
        (KeyCode::Up, Msg::MoveCursorUp),
        (KeyCode::Down, Msg::MoveCursorDown),
        (KeyCode::PageUp, Msg::ScrollPageUp),
        (KeyCode::PageDown, Msg::ScrollPageDown),
        (KeyCode::Backspace, Msg::Backspace),
        (KeyCode::Enter, Msg::Submit),
    ];

    for (code, expected_msg) in test_cases {
        let msg = simulate_key(code, KeyModifiers::NONE, TuiMode::Chat);
        assert_eq!(msg, Some(expected_msg), "{:?} should produce correct Msg", code);
    }
}

#[test]
fn test_character_input_in_chat_mode() {
    for c in ['a', 'b', 'c', 'x', 'y', 'z', ' ', '1', '@'] {
        let msg = simulate_key(KeyCode::Char(c), KeyModifiers::NONE, TuiMode::Chat);
        assert_eq!(msg, Some(Msg::InsertChar(c)), "Char '{}' should produce InsertChar", c);
    }
}

#[test]
fn test_permission_mode_hotkeys() {
    let test_cases = vec![
        (KeyCode::Enter, Msg::PermissionConfirm),
        (KeyCode::Char('y'), Msg::PermissionConfirm),
        (KeyCode::Esc, Msg::PermissionCancel),
        (KeyCode::Char('n'), Msg::PermissionCancel),
        (KeyCode::Char('a'), Msg::PermissionAlways),
        (KeyCode::Char('s'), Msg::PermissionSkip),
    ];

    for (code, expected_msg) in test_cases {
        let msg = simulate_key(code, KeyModifiers::NONE, TuiMode::Permission);
        assert_eq!(msg, Some(expected_msg), "{:?} in Permission mode should produce correct Msg", code);
    }
}

#[test]
fn test_diff_viewer_hotkeys() {
    let test_cases = vec![
        (KeyCode::Esc, Msg::CloseModal),
        (KeyCode::Char('q'), Msg::CloseModal),
        (KeyCode::Down, Msg::ScrollDown),
        (KeyCode::Char('j'), Msg::ScrollDown),
        (KeyCode::Up, Msg::ScrollUp),
        (KeyCode::Char('k'), Msg::ScrollUp),
        (KeyCode::PageDown, Msg::ScrollDown),
        (KeyCode::PageUp, Msg::ScrollUp),
    ];

    for (code, expected_msg) in test_cases {
        let msg = simulate_key(code, KeyModifiers::NONE, TuiMode::DiffViewer);
        assert_eq!(msg, Some(expected_msg), "{:?} in DiffViewer mode should produce correct Msg", code);
    }
}

#[test]
fn test_session_tree_hotkeys() {
    let test_cases = vec![
        (KeyCode::Esc, Msg::CloseModal),
        (KeyCode::Up, Msg::SessionTreeUp),
        (KeyCode::Char('k'), Msg::SessionTreeUp),
        (KeyCode::Down, Msg::SessionTreeDown),
        (KeyCode::Char('j'), Msg::SessionTreeDown),
        (KeyCode::Enter, Msg::SessionTreeConfirm),
    ];

    for (code, expected_msg) in test_cases {
        let msg = simulate_key(code, KeyModifiers::NONE, TuiMode::SessionTree);
        assert_eq!(msg, Some(expected_msg), "{:?} in SessionTree mode should produce correct Msg", code);
    }
}
