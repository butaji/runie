//! Hotkey regression tests - Verify key events produce correct Msgs.

use crossterm::event::{KeyCode, KeyModifiers};
use crate::tui::state::{TuiMode, Msg};
use super::helpers::simulate_key;

#[test]
fn test_ctrl_c_quits_in_chat_mode() {
    let msg = simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::Quit), "Ctrl+C should produce Msg::Quit");
}

#[test]
fn test_ctrl_q_quits_in_chat_mode() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::Quit), "Ctrl+Q should produce Msg::Quit");
}

#[test]
fn test_ctrl_j_inserts_newline_in_chat_mode() {
    // Ctrl+J inserts a newline in the input box
    let msg = simulate_key(KeyCode::Char('j'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert!(matches!(msg, Some(Msg::InsertNewline)), "Ctrl+J should produce Msg::InsertNewline");
}

#[test]
fn test_ctrl_k_scrolls_up_in_chat_mode() {
    let msg = simulate_key(KeyCode::Char('k'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ScrollUp), "Ctrl+K should produce Msg::ScrollUp");
}

#[test]
fn test_ctrl_p_opens_command_palette() {
    let msg = simulate_key(KeyCode::Char('p'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::OpenCommandPalette), "Ctrl+P should produce Msg::OpenCommandPalette");
}

#[test]
fn test_ctrl_b_toggles_sidebar() {
    let msg = simulate_key(KeyCode::Char('b'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ToggleSidebar), "Ctrl+B should produce Msg::ToggleSidebar");
}

#[test]
fn test_enter_submits_in_chat_mode() {
    let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::Submit), "Enter should produce Msg::Submit");
}

#[test]
fn test_page_up_scrolls_in_chat_mode() {
    let msg = simulate_key(KeyCode::PageUp, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ScrollPageUp), "PageUp should produce Msg::ScrollPageUp");
}

#[test]
fn test_page_down_scrolls_in_chat_mode() {
    let msg = simulate_key(KeyCode::PageDown, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ScrollPageDown), "PageDown should produce Msg::ScrollPageDown");
}

#[test]
fn test_character_keys_pass_to_textarea() {
    // Most character input goes to textarea when prompt is focused
    let regular_chars = ['a', 'b', 'c', 'x', 'y', 'z', '1', '@', ' ', 'i'];
    for c in regular_chars {
        let msg = simulate_key(KeyCode::Char(c), KeyModifiers::NONE, TuiMode::Chat);
        assert!(matches!(msg, Some(Msg::TextareaKey(_))), "Char '{}' should produce TextareaKey when prompt focused", c);
    }
}

#[test]
fn test_backspace_pass_to_textarea() {
    let msg = simulate_key(KeyCode::Backspace, KeyModifiers::NONE, TuiMode::Chat);
    assert!(matches!(msg, Some(Msg::TextareaKey(_))), "Backspace should produce TextareaKey");
}

#[test]
fn test_arrow_keys_pass_to_textarea() {
    let msg = simulate_key(KeyCode::Left, KeyModifiers::NONE, TuiMode::Chat);
    assert!(matches!(msg, Some(Msg::TextareaKey(_))), "Left should produce TextareaKey");
    let msg = simulate_key(KeyCode::Right, KeyModifiers::NONE, TuiMode::Chat);
    assert!(matches!(msg, Some(Msg::TextareaKey(_))), "Right should produce TextareaKey");
    let msg = simulate_key(KeyCode::Up, KeyModifiers::NONE, TuiMode::Chat);
    assert!(matches!(msg, Some(Msg::HistoryUp)), "Up should produce HistoryUp");
    let msg = simulate_key(KeyCode::Down, KeyModifiers::NONE, TuiMode::Chat);
    assert!(matches!(msg, Some(Msg::HistoryDown)), "Down should produce HistoryDown");
}

#[test]
fn test_permission_mode_hotkeys() {
    let test_cases = vec![
        (KeyCode::Enter, Msg::PermissionConfirm),
        (KeyCode::Char('y'), Msg::PermissionConfirm),
        (KeyCode::Char('Y'), Msg::PermissionConfirm), // BUG-13 FIX: uppercase Y
        (KeyCode::Esc, Msg::PermissionCancel),
        (KeyCode::Char('n'), Msg::PermissionCancel),
        (KeyCode::Char('N'), Msg::PermissionCancel), // BUG-13 FIX: uppercase N
        (KeyCode::Char('a'), Msg::PermissionAlways),
        (KeyCode::Char('A'), Msg::PermissionAlways), // BUG-13 FIX: uppercase A
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

// P0-4 FIX: Overlay mode hotkeys
// P1-2 FIX: Updated to support model picker navigation
#[test]
fn test_overlay_mode_hotkeys() {
    // Esc should close the overlay
    let msg = simulate_key(KeyCode::Esc, KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::CloseModal), "Esc in Overlay mode should produce Msg::CloseModal");
    
    // Navigation keys should work (model picker support)
    let msg = simulate_key(KeyCode::Up, KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectUp), "Up in Overlay mode should produce Msg::SelectUp");
    
    let msg = simulate_key(KeyCode::Down, KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectDown), "Down in Overlay mode should produce Msg::SelectDown");
    
    let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectConfirm), "Enter in Overlay mode should produce Msg::SelectConfirm");
    
    // j/k navigation
    let msg = simulate_key(KeyCode::Char('j'), KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectDown), "j in Overlay mode should produce Msg::SelectDown");
    
    let msg = simulate_key(KeyCode::Char('k'), KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::SelectUp), "k in Overlay mode should produce Msg::SelectUp");
    
    // Other keys should produce None (not crash)
    let msg = simulate_key(KeyCode::Char('a'), KeyModifiers::NONE, TuiMode::Overlay);
    assert_eq!(msg, None, "Char('a') in Overlay mode should produce None");
}

// P0-1 FIX: Ctrl+C and Ctrl+Q in Permission mode cancel permission
#[test]
fn test_permission_mode_ctrl_keys() {
    // Ctrl+C in Permission mode should cancel
    let msg = simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionCancel), "Ctrl+C in Permission mode should produce Msg::PermissionCancel");
    
    // Ctrl+Q in Permission mode should cancel (P0-3 FIX)
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionCancel), "Ctrl+Q in Permission mode should produce Msg::PermissionCancel");
}
