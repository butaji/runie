//! Tests for global hotkey behavior across modes.
//!
//! Verifies that global hotkeys (Ctrl+Q, Ctrl+C) behave correctly:
//! - Ctrl+Q quits in most modes
//! - Ctrl+Q blocked in Permission (shows cancel)
//! - Ctrl+Q closes Overlay (not quit)
//! - Ctrl+C clears/quits in Chat

use super::*;

/// Test: Ctrl+Q quits in Chat.
#[test]
fn test_ctrl_q_quits_in_chat() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::Quit));
}

/// Test: Ctrl+Q quits in CommandPalette.
#[test]
fn test_ctrl_q_quits_in_palette() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::Quit));
}

/// Test: Ctrl+Q quits in DiffViewer.
#[test]
fn test_ctrl_q_quits_in_diff_viewer() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::DiffViewer);
    // DiffViewer uses CloseModal for Ctrl+Q
    assert_eq!(msg, Some(Msg::CloseModal));
}

/// Test: Ctrl+Q quits in SessionTree.
#[test]
fn test_ctrl_q_quits_in_session_tree() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::SessionTree);
    // SessionTree doesn't specifically intercept Ctrl+Q
    // It goes through route_non_blocking_mode which returns None for Ctrl+Q
    // The actual quit handling would happen at a higher level
    // So this tests the direct key_to_msg output
    // Looking at the code: route_non_blocking_mode doesn't match Ctrl+Q,
    // so it returns None for SessionTree specifically
    // But global_hotkey_handler would catch it before route_non_blocking_mode
    // Actually looking more carefully: global_hotkey_handler is checked FIRST
    // in key_to_msg, before blocking_mode_handler and route_non_blocking_mode
    // So Ctrl+Q should return Quit for SessionTree
    assert_eq!(msg, Some(Msg::Quit));
}

/// Test: Ctrl+Q quits in Onboarding.
#[test]
fn test_ctrl_q_quits_in_onboarding() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Onboarding);
    assert_eq!(msg, Some(Msg::Quit));
}

/// Test: Ctrl+Q blocked in Permission (shows cancel).
#[test]
fn test_ctrl_q_blocked_in_permission() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionCancel), "Ctrl+Q in Permission should cancel, not quit");
}

/// Test: Ctrl+Q closes Overlay (not quit).
#[test]
fn test_ctrl_q_closes_overlay() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Overlay);
    assert_eq!(msg, Some(Msg::CloseModal), "Ctrl+Q in Overlay should close, not quit");
}

/// Test: Ctrl+C in Chat with empty textarea quits.
#[test]
fn test_ctrl_c_quits_empty_chat() {
    let event = Event::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let state = AppState {
        mode: TuiMode::Chat,
        textarea: TextArea::default(),
        ..Default::default()
    };
    let msgs = event_to_msg(event, &state);
    assert!(msgs.contains(&Msg::Quit), "Ctrl+C with empty textarea should quit");
}

/// Test: Ctrl+C in Chat with text shows clear input.
#[test]
fn test_ctrl_c_clears_input_with_text() {
    let event = Event::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let state = AppState {
        mode: TuiMode::Chat,
        textarea: TextArea::new(vec!["some text".to_string()]),
        ..Default::default()
    };
    let msgs = event_to_msg(event, &state);
    assert!(msgs.contains(&Msg::ClearInputConfirm), "Ctrl+C with text should clear input");
}

/// Test: Ctrl+C blocked in Permission (shows cancel).
#[test]
fn test_ctrl_c_blocked_in_permission() {
    let msg = simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Permission);
    assert_eq!(msg, Some(Msg::PermissionCancel), "Ctrl+C in Permission should cancel, not quit");
}

/// Test: Ctrl+C in Onboarding quits.
#[test]
fn test_ctrl_c_quits_in_onboarding() {
    let event = Event::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let state = AppState {
        mode: TuiMode::Onboarding,
        textarea: TextArea::default(),
        ..Default::default()
    };
    let msgs = event_to_msg(event, &state);
    // In Onboarding with empty textarea, Ctrl+C produces Quit
    assert!(msgs.contains(&Msg::Quit));
}

/// Test: Ctrl+M switches model.
#[test]
fn test_ctrl_m_switches_model() {
    let msg = simulate_key(KeyCode::Char('m'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::SwitchModel));
}

/// Test: Ctrl+K opens command palette.
#[test]
fn test_ctrl_k_opens_palette() {
    let msg = simulate_key(KeyCode::Char('k'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::OpenCommandPalette));
}

/// Test: Ctrl+J inserts newline.
#[test]
fn test_ctrl_j_inserts_newline() {
    let msg = simulate_key(KeyCode::Char('j'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::InsertNewline));
}

/// Test: Ctrl+B toggles sidebar.
#[test]
fn test_ctrl_b_toggles_sidebar() {
    let msg = simulate_key(KeyCode::Char('b'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ToggleSidebar));
}

/// Test: Ctrl+L clears chat.
#[test]
fn test_ctrl_l_clears_chat() {
    let msg = simulate_key(KeyCode::Char('l'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ClearChat));
}

/// Test: Ctrl+O copies last response.
#[test]
fn test_ctrl_o_copies_last_response() {
    let msg = simulate_key(KeyCode::Char('o'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::CopyLastResponse));
}

/// Test: Enter submits in Chat.
#[test]
fn test_enter_submits_in_chat() {
    let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::Submit));
}

/// Test: Shift+Enter inserts newline in Chat.
#[test]
fn test_shift_enter_inserts_newline() {
    let event = Event::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::SHIFT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let state = AppState {
        mode: TuiMode::Chat,
        ..Default::default()
    };
    let msgs = event_to_msg(event, &state);
    assert!(msgs.contains(&Msg::InsertNewline));
}

/// Test: Arrow Up navigates history in Chat.
#[test]
fn test_arrow_up_navigates_history() {
    let msg = simulate_key(KeyCode::Up, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::HistoryUp));
}

/// Test: Arrow Down navigates history in Chat.
#[test]
fn test_arrow_down_navigates_history() {
    let msg = simulate_key(KeyCode::Down, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::HistoryDown));
}

/// Test: PageUp scrolls in Chat.
#[test]
fn test_page_up_scrolls() {
    let msg = simulate_key(KeyCode::PageUp, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ScrollPageUp));
}

/// Test: PageDown scrolls in Chat.
#[test]
fn test_page_down_scrolls() {
    let msg = simulate_key(KeyCode::PageDown, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ScrollPageDown));
}

/// Test: No hotkeys work in Permission except those handled by blocking_mode_handler.
#[test]
fn test_no_global_hotkeys_in_permission() {
    // Ctrl+Q, Ctrl+C are intercepted by blocking_mode_handler
    assert_eq!(
        simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Permission),
        Some(Msg::PermissionCancel)
    );
    assert_eq!(
        simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Permission),
        Some(Msg::PermissionCancel)
    );

    // Other Ctrl combos should be blocked (return None)
    assert_eq!(
        simulate_key(KeyCode::Char('k'), KeyModifiers::CONTROL, TuiMode::Permission),
        None
    );
    assert_eq!(
        simulate_key(KeyCode::Char('l'), KeyModifiers::CONTROL, TuiMode::Permission),
        None
    );
}

/// Test: No hotkeys work in Overlay except those handled by blocking_mode_handler.
#[test]
fn test_no_global_hotkeys_in_overlay() {
    // Ctrl+Q is intercepted by blocking_mode_handler
    assert_eq!(
        simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Overlay),
        Some(Msg::CloseModal)
    );

    // Other Ctrl combos should be blocked (return None)
    assert_eq!(
        simulate_key(KeyCode::Char('k'), KeyModifiers::CONTROL, TuiMode::Overlay),
        None
    );
    assert_eq!(
        simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Overlay),
        None
    );
}
