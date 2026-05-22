//! Comprehensive hotkey tests to prevent keyboard shortcuts from breaking.
//!
//! Tests cover:
//! - Input Box Hotkeys (Ctrl+key while in Chat mode)
//! - App-Wide Hotkeys (global, regardless of mode)

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests_hotkeys {
    use crossterm::event::{Event, KeyCode, KeyModifiers, KeyEvent, KeyEventKind, KeyEventState};
    use crate::tui::state::{AppState, TuiMode, Msg};
    use crate::tui::update::update;
    use crate::tui::events::event_to_msg;

    /// Helper: simulate a keyboard event and return the resulting Msg
    fn simulate_key(code: KeyCode, modifiers: KeyModifiers, mode: TuiMode) -> Option<Msg> {
        let event = Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });
        let state = AppState {
            mode,
            ..Default::default()
        };
        event_to_msg(event, &state)
    }

    /// Helper: create AppState in Chat mode with some input typed
    fn make_chat_state_with_input(text: &str) -> AppState {
        let mut state = AppState {
            mode: TuiMode::Chat,
            ..Default::default()
        };
        for c in text.chars() {
            update(&mut state, Msg::InsertChar(c));
        }
        state
    }

    /// Helper: create AppState in CommandPalette mode
    fn make_palette_state() -> AppState {
        let mut state = AppState::default();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;
        state
    }

    /// Helper: create AppState with modal open
    fn make_state_with_modal(mode: TuiMode) -> AppState {
        let mut state = AppState {
            mode: mode.clone(),
            ..Default::default()
        };
        if mode == TuiMode::CommandPalette {
            state.command_palette.open = true;
        }
        state
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // INPUT BOX HOTKEYS (Ctrl+key while in Chat mode)
    // ═══════════════════════════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════════════════════════
    // APP-WIDE HOTKEYS (global, regardless of mode)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_esc_closes_modal() {
        // Test in CommandPalette mode
        let msg = simulate_key(KeyCode::Esc, KeyModifiers::NONE, TuiMode::CommandPalette);
        assert_eq!(msg, Some(Msg::CloseModal), "Esc in CommandPalette should produce Msg::CloseModal");

        // Test in DiffViewer mode
        let msg = simulate_key(KeyCode::Esc, KeyModifiers::NONE, TuiMode::DiffViewer);
        assert_eq!(msg, Some(Msg::CloseModal), "Esc in DiffViewer should produce Msg::CloseModal");

        // Test in SessionTree mode
        let msg = simulate_key(KeyCode::Esc, KeyModifiers::NONE, TuiMode::SessionTree);
        assert_eq!(msg, Some(Msg::CloseModal), "Esc in SessionTree should produce Msg::CloseModal");

        // Verify state update
        let mut state = make_state_with_modal(TuiMode::CommandPalette);
        state.command_palette.open = true;
        update(&mut state, Msg::CloseModal);
        assert!(!state.command_palette.open, "CloseModal should close command palette");
        assert_eq!(state.mode, TuiMode::Chat, "Mode should return to Chat");
    }

    #[test]
    fn test_enter_submits_in_chat() {
        let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::Chat);
        assert_eq!(msg, Some(Msg::Submit), "Enter in Chat mode should produce Msg::Submit");

        // Verify state update - empty input should not submit
        let mut state = AppState {
            mode: TuiMode::Chat,
            ..Default::default()
        };
        let cmds = update(&mut state, Msg::Submit);
        assert!(cmds.is_empty(), "Submit with empty input should produce no commands");
        assert_eq!(state.messages.len(), 0, "No message should be added");

        // Verify state update - non-empty input should submit
        let mut state = make_chat_state_with_input("hello");
        let cmds = update(&mut state, Msg::Submit);
        assert!(!cmds.is_empty(), "Submit with input should produce commands");
        assert_eq!(state.messages.len(), 1, "One message should be added");
        assert_eq!(state.input_lines, vec![""], "Input should be cleared");
    }

    #[test]
    fn test_enter_selects_in_palette() {
        let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::CommandPalette);
        assert_eq!(msg, Some(Msg::CommandPaletteConfirm), "Enter in CommandPalette should produce Msg::CommandPaletteConfirm");

        // Verify state update
        let mut state = make_state_with_modal(TuiMode::CommandPalette);
        state.command_palette.open = true;
        update(&mut state, Msg::CommandPaletteConfirm);
        assert!(!state.command_palette.open, "CommandPaletteConfirm should close palette");
        assert_eq!(state.mode, TuiMode::Chat, "Mode should return to Chat");
    }

    #[test]
    fn test_up_down_navigate_palette() {
        // Up navigation
        let msg = simulate_key(KeyCode::Up, KeyModifiers::NONE, TuiMode::CommandPalette);
        assert_eq!(msg, Some(Msg::CommandPaletteUp), "Up in CommandPalette should produce Msg::CommandPaletteUp");

        // Down navigation
        let msg = simulate_key(KeyCode::Down, KeyModifiers::NONE, TuiMode::CommandPalette);
        assert_eq!(msg, Some(Msg::CommandPaletteDown), "Down in CommandPalette should produce Msg::CommandPaletteDown");

        // Verify state updates
        let mut state = make_state_with_modal(TuiMode::CommandPalette);
        state.command_palette.selected = 5;

        update(&mut state, Msg::CommandPaletteUp);
        assert_eq!(state.command_palette.selected, 4, "CommandPaletteUp should decrement selection");

        update(&mut state, Msg::CommandPaletteDown);
        assert_eq!(state.command_palette.selected, 5, "CommandPaletteDown should increment selection");

        // Test boundary - should not go below 0
        state.command_palette.selected = 0;
        update(&mut state, Msg::CommandPaletteUp);
        assert_eq!(state.command_palette.selected, 0, "CommandPaletteUp should not go below 0");
    }

    #[test]
    fn test_page_up_down_scrolls() {
        // PageUp
        let msg = simulate_key(KeyCode::PageUp, KeyModifiers::NONE, TuiMode::Chat);
        assert_eq!(msg, Some(Msg::ScrollPageUp), "PageUp should produce Msg::ScrollPageUp");

        // PageDown
        let msg = simulate_key(KeyCode::PageDown, KeyModifiers::NONE, TuiMode::Chat);
        assert_eq!(msg, Some(Msg::ScrollPageDown), "PageDown should produce Msg::ScrollPageDown");

        // Verify state updates
        let mut state = AppState {
            mode: TuiMode::Chat,
            messages: (0..20).map(|i| crate::components::MessageItem::User {
                text: format!("message {}", i),
                model: Some("You".to_string()),
                timestamp: None,
            }).collect(),
            scroll: crate::tui::state::ScrollState::default(),
            ..Default::default()
        };

        // Scroll down
        update(&mut state, Msg::ScrollPageDown);
        assert!(state.scroll.feed_offset > 0, "ScrollPageDown should increase offset");

        let offset_after_down = state.scroll.feed_offset;

        // Scroll up
        update(&mut state, Msg::ScrollPageUp);
        assert!(state.scroll.feed_offset < offset_after_down, "ScrollPageUp should decrease offset");

        // Test saturation at boundaries
        state.scroll.feed_offset = 0;
        update(&mut state, Msg::ScrollPageUp);
        assert_eq!(state.scroll.feed_offset, 0, "Scroll should not go below 0");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HOTKEY REGRESSION TESTS - Verify key events produce correct Msgs
    // ═══════════════════════════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════════════════════════
    // STATE CONSISTENCY TESTS - Verify dirty flag is set correctly
    // ═══════════════════════════════════════════════════════════════════════════

    /// Mock Tui for dirty flag testing
    struct MockTui {
        state: AppState,
        dirty: bool,
    }

    impl MockTui {
        fn new() -> Self {
            Self {
                state: AppState::default(),
                dirty: false,
            }
        }

        fn update(&mut self, msg: Msg) {
            self.dirty = true;
            update(&mut self.state, msg);
        }

        fn is_dirty(&self) -> bool {
            self.dirty
        }

        fn clear_dirty(&mut self) {
            self.dirty = false;
        }
    }

    #[test]
    fn test_hotkey_updates_set_dirty() {
        let mut tui = MockTui::new();

        let hotkey_cases = vec![
            (KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Chat),
            (KeyCode::Char('j'), KeyModifiers::CONTROL, TuiMode::Chat),
            (KeyCode::Char('a'), KeyModifiers::CONTROL, TuiMode::Chat),
            (KeyCode::Char('e'), KeyModifiers::CONTROL, TuiMode::Chat),
            (KeyCode::Char('w'), KeyModifiers::CONTROL, TuiMode::Chat),
            (KeyCode::Char('u'), KeyModifiers::CONTROL, TuiMode::Chat),
            (KeyCode::Char('d'), KeyModifiers::CONTROL, TuiMode::Chat),
            (KeyCode::Char('b'), KeyModifiers::CONTROL, TuiMode::Chat),
            (KeyCode::Char('k'), KeyModifiers::CONTROL, TuiMode::Chat),
            (KeyCode::Esc, KeyModifiers::NONE, TuiMode::CommandPalette),
            (KeyCode::Enter, KeyModifiers::NONE, TuiMode::CommandPalette),
            (KeyCode::Up, KeyModifiers::NONE, TuiMode::CommandPalette),
            (KeyCode::Down, KeyModifiers::NONE, TuiMode::CommandPalette),
            (KeyCode::Enter, KeyModifiers::NONE, TuiMode::Chat),
            (KeyCode::PageUp, KeyModifiers::NONE, TuiMode::Chat),
            (KeyCode::PageDown, KeyModifiers::NONE, TuiMode::Chat),
        ];

        for (code, modifiers, mode) in hotkey_cases {
            tui.clear_dirty();
            let event = Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            });
            let state = AppState { mode: mode.clone(), ..Default::default() };

            if let Some(msg) = event_to_msg(event, &state) {
                tui.update(msg);
                assert!(tui.is_dirty(), "Hotkey {:?}+{:?} in {:?} mode should set dirty", modifiers, code, mode);
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MODE-SPECIFIC BEHAVIOR TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_ctrl_keys_dont_work_in_permission_mode() {
        // Ctrl+C, Ctrl+Q are NOT handled in Permission mode (returns None)
        // Permission mode only handles Enter, Esc, y, n, a, s
        let state = AppState {
            mode: TuiMode::Permission,
            ..Default::default()
        };

        let event = Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });

        let msg = event_to_msg(event, &state);
        assert_eq!(msg, None, "Ctrl+C in Permission is not handled (returns None)");
    }

    #[test]
    fn test_enter_doesnt_submit_in_permission_mode() {
        let state = AppState {
            mode: TuiMode::Permission,
            ..Default::default()
        };

        let event = Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });

        let msg = event_to_msg(event, &state);
        assert_eq!(msg, Some(Msg::PermissionConfirm), "Enter in Permission should confirm, not submit");
    }

    #[test]
    fn test_ctrl_keys_dont_work_in_palette_mode() {
        let state = AppState {
            mode: TuiMode::CommandPalette,
            command_palette: crate::tui::state::CommandPaletteState {
                open: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Ctrl+B is treated as regular 'b' char in palette mode (goes to filter)
        // This is because key_to_palette_msg doesn't check control modifiers on Char
        let event = Event::Key(KeyEvent {
            code: KeyCode::Char('b'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });

        let msg = event_to_msg(event, &state);
        assert_eq!(msg, Some(Msg::CommandPaletteFilter('b')), "Ctrl+B in CommandPalette treated as filter 'b'");
    }

    #[test]
    fn test_arrow_keys_dont_affect_input_in_palette_mode() {
        let state = AppState {
            mode: TuiMode::CommandPalette,
            command_palette: crate::tui::state::CommandPaletteState {
                open: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Left/Right should not move cursor in palette mode
        let event = Event::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });

        let msg = event_to_msg(event, &state);
        assert_eq!(msg, None, "Left in CommandPalette should be ignored");
    }
}
