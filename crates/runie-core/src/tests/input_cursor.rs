//! Tests for input cursor movement (Emacs-style hotkeys)

#[cfg(test)]
mod tests {
    use crate::event::{Event, InputEvent};
    use crate::model::AppState;

    #[test]
    fn cursor_starts_at_zero() {
        let state = AppState::default();
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn insert_char_moves_cursor_forward() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('h'));
        state.update(InputEvent::Input('i'));
        assert_eq!(state.input.input, "hi");
        assert_eq!(state.input.cursor_pos, 2);
    }

    #[test]
    fn cursor_left_moves_cursor_back() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::Input('b'));
        state.update(InputEvent::CursorLeft);
        assert_eq!(state.input.cursor_pos, 1);
    }

    #[test]
    fn cursor_left_at_start_does_nothing() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::CursorLeft);
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn cursor_right_moves_cursor_forward() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::CursorRight);
        assert_eq!(state.input.cursor_pos, 1);
    }

    #[test]
    fn cursor_right_at_end_does_nothing() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::CursorRight);
        assert_eq!(state.input.cursor_pos, 1);
    }

    #[test]
    fn cursor_start_moves_to_beginning() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::Input('b'));
        state.update(InputEvent::Input('c'));
        state.update(InputEvent::CursorStart);
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn cursor_end_moves_to_end() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::Input('b'));
        state.update(InputEvent::Input('c'));
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::CursorEnd);
        assert_eq!(state.input.cursor_pos, 3);
    }

    #[test]
    fn backspace_deletes_before_cursor_and_moves_left() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::Input('b'));
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::Backspace);
        // Deleted 'a', cursor moves left
        assert_eq!(state.input.input, "b");
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn backspace_at_start_does_nothing() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::CursorStart);
        state.update(InputEvent::Backspace);
        assert_eq!(state.input.input, "a");
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn delete_word_removes_word_before_cursor() {
        let mut state = AppState::default();
        // Type "hello world"
        for c in "hello world".chars() {
            state.update(InputEvent::Input(c));
        }
        // Move to end
        state.update(InputEvent::CursorEnd);
        // Delete word (Emacs: deletes previous word "world")
        state.update(InputEvent::DeleteWord);
        assert_eq!(state.input.input, "hello ");
        assert_eq!(state.input.cursor_pos, 6);
    }

    #[test]
    fn delete_word_at_middle_of_word() {
        let mut state = AppState::default();
        // Type "hello world"
        for c in "hello world".chars() {
            state.update(InputEvent::Input(c));
        }
        // Move to position 8 (before 'r' in "world")
        state.update(InputEvent::CursorEnd); // at 11
        state.update(InputEvent::CursorLeft); // at 10
        state.update(InputEvent::CursorLeft); // at 9
        state.update(InputEvent::CursorLeft); // at 8 (before 'r')
                                              // DeleteWord should delete from position 6 ("w") to cursor 8 = "wo"
        state.update(InputEvent::DeleteWord);
        assert_eq!(state.input.input, "hello rld");
        assert_eq!(state.input.cursor_pos, 6);
    }

    #[test]
    fn delete_to_end_removes_from_cursor_to_end() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('h'));
        state.update(InputEvent::Input('e'));
        state.update(InputEvent::Input('l'));
        state.update(InputEvent::Input('l'));
        state.update(InputEvent::Input('o'));
        // Move cursor left 3 times to position 2 (after "he")
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::DeleteToEnd);
        // Should delete "llo", leaving "he"
        assert_eq!(state.input.input, "he");
        assert_eq!(state.input.cursor_pos, 2);
    }

    #[test]
    fn delete_to_start_removes_from_start_to_cursor() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('h'));
        state.update(InputEvent::Input('e'));
        state.update(InputEvent::Input('l'));
        state.update(InputEvent::Input('l'));
        state.update(InputEvent::Input('o'));
        // Move cursor to middle
        state.update(InputEvent::CursorEnd);
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::DeleteToStart);
        // Should delete "he", leaving "lo"
        assert_eq!(state.input.input, "lo");
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn kill_char_removes_char_after_cursor() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::Input('b'));
        state.update(InputEvent::Input('c'));
        // Cursor at end, KillChar is no-op
        state.update(InputEvent::KillChar);
        assert_eq!(state.input.input, "abc");
        // Move cursor left twice (between 'a' and 'b')
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::CursorLeft);
        // KillChar deletes 'b' (char after cursor)
        state.update(InputEvent::KillChar);
        assert_eq!(state.input.input, "ac");
        assert_eq!(state.input.cursor_pos, 1);
    }

    #[test]
    fn kill_char_at_end_does_nothing() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::KillChar);
        assert_eq!(state.input.input, "a");
        assert_eq!(state.input.cursor_pos, 1);
    }

    #[test]
    fn submit_clears_input_and_resets_cursor() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('t'));
        state.update(InputEvent::Input('e'));
        state.update(InputEvent::Input('s'));
        state.update(InputEvent::Input('t'));
        assert_eq!(state.input.input, "test");
        assert_eq!(state.input.cursor_pos, 4);
        state.update(Event::submit());
        assert!(state.input.input.is_empty());
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn insert_char_at_middle_position() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::Input('c'));
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::Input('b'));
        assert_eq!(state.input.input, "abc");
        assert_eq!(state.input.cursor_pos, 2);
    }
}
