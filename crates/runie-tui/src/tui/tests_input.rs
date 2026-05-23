#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests_input {
    use crate::tui::state::{AppState, Msg};
    use crate::tui::update::update;

    fn make_state() -> AppState {
        let mut state = AppState::default();
        state.input_lines = vec![String::new()];
        state
    }

    #[test]
    fn test_insert_ascii_char() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('a'));
        assert_eq!(state.input_lines[0], "a");
        assert_eq!(state.cursor_col, 1);
    }

    #[test]
    fn test_insert_multibyte_char() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('❯'));
        assert_eq!(state.input_lines[0], "❯");
        assert_eq!(state.cursor_col, 1);
        
        update(&mut state, Msg::InsertChar('a'));
        assert_eq!(state.input_lines[0], "❯a");
        assert_eq!(state.cursor_col, 2);
    }

    #[test]
    fn test_insert_emoji() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('😀'));
        assert_eq!(state.input_lines[0], "😀");
        assert_eq!(state.cursor_col, 1);
        
        update(&mut state, Msg::InsertChar('b'));
        assert_eq!(state.input_lines[0], "😀b");
        assert_eq!(state.cursor_col, 2);
    }

    #[test]
    fn test_backspace_multibyte() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('❯'));
        update(&mut state, Msg::InsertChar('a'));
        assert_eq!(state.input_lines[0], "❯a");
        
        update(&mut state, Msg::Backspace);
        assert_eq!(state.input_lines[0], "❯");
        assert_eq!(state.cursor_col, 1);
        
        update(&mut state, Msg::Backspace);
        assert_eq!(state.input_lines[0], "");
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_delete_forward_multibyte() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('a'));
        update(&mut state, Msg::InsertChar('❯'));
        update(&mut state, Msg::InsertChar('b'));
        assert_eq!(state.input_lines[0], "a❯b");
        
        // Move cursor to before '❯'
        state.cursor_col = 1;
        update(&mut state, Msg::DeleteForward);
        assert_eq!(state.input_lines[0], "ab");
        assert_eq!(state.cursor_col, 1);
    }

    #[test]
    fn test_cursor_move_with_multibyte() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('❯'));
        update(&mut state, Msg::InsertChar('a'));
        update(&mut state, Msg::InsertChar('b'));
        assert_eq!(state.cursor_col, 3); // 3 chars
        
        update(&mut state, Msg::MoveCursorToStart);
        assert_eq!(state.cursor_col, 0);
        
        update(&mut state, Msg::MoveCursorToEnd);
        assert_eq!(state.cursor_col, 3); // 3 chars, not bytes
    }

    #[test]
    fn test_delete_word_backward_multibyte() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        update(&mut state, Msg::InsertChar(' '));
        update(&mut state, Msg::InsertChar('❯'));
        assert_eq!(state.input_lines[0], "hi ❯");
        assert_eq!(state.cursor_col, 4);
        
        // DeleteWordBackward removes word + preceding space (Unix convention)
        update(&mut state, Msg::DeleteWordBackward);
        assert_eq!(state.input_lines[0], "hi");
        assert_eq!(state.cursor_col, 2);
    }

    #[test]
    fn test_delete_to_start_multibyte() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('❯'));
        update(&mut state, Msg::InsertChar('a'));
        update(&mut state, Msg::InsertChar('b'));
        assert_eq!(state.input_lines[0], "❯ab");
        
        state.cursor_col = 2; // between 'a' and 'b'
        update(&mut state, Msg::DeleteToStart);
        assert_eq!(state.input_lines[0], "b");
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_insert_newline_multibyte() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('a'));
        update(&mut state, Msg::InsertChar('❯'));
        update(&mut state, Msg::InsertChar('b'));
        state.cursor_col = 2; // after '❯'
        
        update(&mut state, Msg::InsertNewline);
        assert_eq!(state.input_lines[0], "a❯");
        assert_eq!(state.input_lines[1], "b");
        assert_eq!(state.cursor_row, 1);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_move_right_with_multibyte() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('❯'));
        update(&mut state, Msg::InsertChar('a'));
        state.cursor_col = 0;
        
        update(&mut state, Msg::MoveCursorRight);
        assert_eq!(state.cursor_col, 1);
        
        update(&mut state, Msg::MoveCursorRight);
        assert_eq!(state.cursor_col, 2);
    }

    #[test]
    fn test_move_left_with_multibyte() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('❯'));
        update(&mut state, Msg::InsertChar('a'));
        state.cursor_col = 2;
        
        update(&mut state, Msg::MoveCursorLeft);
        assert_eq!(state.cursor_col, 1);
        
        update(&mut state, Msg::MoveCursorLeft);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_rapid_insert_no_panic() {
        let mut state = make_state();
        // Simulate rapid typing of mixed ASCII and multi-byte
        let chars = vec!['a', '❯', 'b', '😀', 'c', '⠿', 'd'];
        for c in chars {
            update(&mut state, Msg::InsertChar(c));
        }
        assert_eq!(state.input_lines[0], "a❯b😀c⠿d");
        assert_eq!(state.cursor_col, 7);
    }

    #[test]
    fn test_rapid_backspace_no_panic() {
        let mut state = make_state();
        let chars = vec!['a', '❯', 'b', '😀'];
        for c in chars {
            update(&mut state, Msg::InsertChar(c));
        }
        // Rapid backspace
        for _ in 0..4 {
            update(&mut state, Msg::Backspace);
        }
        assert_eq!(state.input_lines[0], "");
        assert_eq!(state.cursor_col, 0);
    }
}