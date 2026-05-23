//! Input Box Hotkeys tests (Ctrl+key while in Chat mode).

use crossterm::event::{KeyCode, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg};
use crate::tui::update::update;
use super::helpers::{simulate_key, make_chat_state_with_input};

#[test]
fn test_ctrl_c_quits() {
    let msg = simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::Quit), "Ctrl+C should produce Msg::Quit");

    // Verify state update
    let mut state = make_chat_state_with_input("hello");
    update(&mut state, Msg::Quit);
    assert!(!state.running, "Quit should set running=false");
}

#[test]
fn test_ctrl_q_quits() {
    let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::Quit), "Ctrl+Q should produce Msg::Quit");

    // Verify state update
    let mut state = make_chat_state_with_input("hello");
    update(&mut state, Msg::Quit);
    assert!(!state.running, "Quit should set running=false");
}

#[test]
fn test_ctrl_j_newline() {
    let msg = simulate_key(KeyCode::Char('j'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::InsertNewline), "Ctrl+J should produce Msg::InsertNewline");

    // Verify state update
    let mut state = make_chat_state_with_input("hello");
    let line_count_before = state.input_lines.len();
    update(&mut state, Msg::InsertNewline);
    assert_eq!(state.input_lines.len(), line_count_before + 1, "InsertNewline should add new line");
    assert_eq!(state.cursor_row, 1, "Cursor should move to new line");
    assert_eq!(state.cursor_col, 0, "Cursor should be at start of new line");
}

#[test]
fn test_ctrl_a_start_of_line() {
    let msg = simulate_key(KeyCode::Char('a'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::MoveCursorToStart), "Ctrl+A should produce Msg::MoveCursorToStart");

    // Verify state update
    let mut state = make_chat_state_with_input("hello");
    state.cursor_col = 5; // Move to end
    update(&mut state, Msg::MoveCursorToStart);
    assert_eq!(state.cursor_col, 0, "MoveCursorToStart should move cursor to column 0");
}

#[test]
fn test_ctrl_e_end_of_line() {
    let msg = simulate_key(KeyCode::Char('e'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::MoveCursorToEnd), "Ctrl+E should produce Msg::MoveCursorToEnd");

    // Verify state update
    let mut state = make_chat_state_with_input("hello");
    state.cursor_col = 0; // Move to start
    update(&mut state, Msg::MoveCursorToEnd);
    assert_eq!(state.cursor_col, 5, "MoveCursorToEnd should move cursor to end of line");
}

#[test]
fn test_ctrl_w_delete_word() {
    let msg = simulate_key(KeyCode::Char('w'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::DeleteWordBackward), "Ctrl+W should produce Msg::DeleteWordBackward");

    // Verify state update
    let mut state = make_chat_state_with_input("hello world");
    state.cursor_col = 11; // At end
    update(&mut state, Msg::DeleteWordBackward);
    assert_eq!(state.input_lines[0], "hello", "DeleteWordBackward should delete word before cursor");
    assert_eq!(state.cursor_col, 5, "Cursor should be at end of remaining text");
}

#[test]
fn test_ctrl_u_delete_to_start() {
    let msg = simulate_key(KeyCode::Char('u'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::DeleteToStart), "Ctrl+U should produce Msg::DeleteToStart");

    // Verify state update
    let mut state = make_chat_state_with_input("hello");
    state.cursor_col = 3;
    update(&mut state, Msg::DeleteToStart);
    assert_eq!(state.input_lines[0], "lo", "DeleteToStart should delete from cursor to start");
    assert_eq!(state.cursor_col, 0, "Cursor should be at position 0");
}

#[test]
fn test_ctrl_d_delete_forward() {
    let msg = simulate_key(KeyCode::Char('d'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::DeleteForward), "Ctrl+D should produce Msg::DeleteForward");

    // Verify state update
    let mut state = make_chat_state_with_input("hello");
    state.cursor_col = 0;
    update(&mut state, Msg::DeleteForward);
    assert_eq!(state.input_lines[0], "ello", "DeleteForward should delete char at cursor");
    assert_eq!(state.cursor_col, 0, "Cursor should remain at same position");
}

#[test]
fn test_ctrl_b_toggles_sidebar() {
    let msg = simulate_key(KeyCode::Char('b'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ToggleSidebar), "Ctrl+B should produce Msg::ToggleSidebar");

    // Verify state update
    let mut state = make_chat_state_with_input("hello");
    assert!(!state.show_sidebar, "Sidebar should start hidden");
    update(&mut state, Msg::ToggleSidebar);
    assert!(state.show_sidebar, "ToggleSidebar should show sidebar");
    update(&mut state, Msg::ToggleSidebar);
    assert!(!state.show_sidebar, "ToggleSidebar should hide sidebar again");
}

#[test]
fn test_ctrl_k_opens_palette() {
    let msg = simulate_key(KeyCode::Char('k'), KeyModifiers::CONTROL, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::OpenCommandPalette), "Ctrl+K should produce Msg::OpenCommandPalette");

    // Verify state update
    let mut state = make_chat_state_with_input("hello");
    update(&mut state, Msg::OpenCommandPalette);
    assert!(state.command_palette.open, "OpenCommandPalette should open palette");
    assert_eq!(state.mode, TuiMode::CommandPalette, "Mode should switch to CommandPalette");
    assert_eq!(state.command_palette.filter, "", "Filter should be cleared");
    assert_eq!(state.command_palette.selected, 0, "Selection should reset to 0");
}
