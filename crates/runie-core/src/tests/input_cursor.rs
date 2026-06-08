//! Tests for input cursor movement (Emacs-style hotkeys)

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::event::Event;

    #[test]
    fn cursor_starts_at_zero() {
        let state = AppState::default();
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn insert_char_moves_cursor_forward() {
        let mut state = AppState::default();
        state.update(Event::Input('h'));
        state.update(Event::Input('i'));
        assert_eq!(state.input, "hi");
        assert_eq!(state.cursor_pos, 2);
    }

    #[test]
    fn cursor_left_moves_cursor_back() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::CursorLeft);
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn cursor_left_at_start_does_nothing() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::CursorLeft);
        state.update(Event::CursorLeft);
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn cursor_right_moves_cursor_forward() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::CursorLeft);
        state.update(Event::CursorRight);
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn cursor_right_at_end_does_nothing() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::CursorRight);
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn cursor_start_moves_to_beginning() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::Input('c'));
        state.update(Event::CursorStart);
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn cursor_end_moves_to_end() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::Input('c'));
        state.update(Event::CursorLeft);
        state.update(Event::CursorLeft);
        state.update(Event::CursorEnd);
        assert_eq!(state.cursor_pos, 3);
    }

    #[test]
    fn backspace_deletes_before_cursor_and_moves_left() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::CursorLeft);
        state.update(Event::Backspace);
        // Deleted 'a', cursor moves left
        assert_eq!(state.input, "b");
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn backspace_at_start_does_nothing() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::CursorStart);
        state.update(Event::Backspace);
        assert_eq!(state.input, "a");
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn delete_word_removes_word_before_cursor() {
        let mut state = AppState::default();
        // Type "hello world"
        for c in "hello world".chars() {
            state.update(Event::Input(c));
        }
        // Move to end
        state.update(Event::CursorEnd);
        // Delete word (Emacs: deletes previous word "world")
        state.update(Event::DeleteWord);
        assert_eq!(state.input, "hello ");
        assert_eq!(state.cursor_pos, 6);
    }

    #[test]
    fn delete_word_at_middle_of_word() {
        let mut state = AppState::default();
        // Type "hello world"
        for c in "hello world".chars() {
            state.update(Event::Input(c));
        }
        // Move to position 8 (before 'r' in "world")
        state.update(Event::CursorEnd); // at 11
        state.update(Event::CursorLeft); // at 10
        state.update(Event::CursorLeft); // at 9
        state.update(Event::CursorLeft); // at 8 (before 'r')
        // DeleteWord should delete from position 6 ("w") to cursor 8 = "wo"
        state.update(Event::DeleteWord);
        assert_eq!(state.input, "hello rld");
        assert_eq!(state.cursor_pos, 6);
    }

    #[test]
    fn delete_to_end_removes_from_cursor_to_end() {
        let mut state = AppState::default();
        state.update(Event::Input('h'));
        state.update(Event::Input('e'));
        state.update(Event::Input('l'));
        state.update(Event::Input('l'));
        state.update(Event::Input('o'));
        // Move cursor left 3 times to position 2 (after "he")
        state.update(Event::CursorLeft);
        state.update(Event::CursorLeft);
        state.update(Event::CursorLeft);
        state.update(Event::DeleteToEnd);
        // Should delete "llo", leaving "he"
        assert_eq!(state.input, "he");
        assert_eq!(state.cursor_pos, 2);
    }

    #[test]
    fn delete_to_start_removes_from_start_to_cursor() {
        let mut state = AppState::default();
        state.update(Event::Input('h'));
        state.update(Event::Input('e'));
        state.update(Event::Input('l'));
        state.update(Event::Input('l'));
        state.update(Event::Input('o'));
        // Move cursor to middle
        state.update(Event::CursorEnd);
        state.update(Event::CursorLeft);
        state.update(Event::CursorLeft);
        state.update(Event::DeleteToStart);
        // Should delete "he", leaving "lo"
        assert_eq!(state.input, "lo");
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn kill_char_removes_char_after_cursor() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::Input('c'));
        // Cursor at end, KillChar is no-op
        state.update(Event::KillChar);
        assert_eq!(state.input, "abc");
        // Move cursor left twice (between 'a' and 'b')
        state.update(Event::CursorLeft);
        state.update(Event::CursorLeft);
        // KillChar deletes 'b' (char after cursor)
        state.update(Event::KillChar);
        assert_eq!(state.input, "ac");
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn kill_char_at_end_does_nothing() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::KillChar);
        assert_eq!(state.input, "a");
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn submit_clears_input_and_resets_cursor() {
        let mut state = AppState::default();
        state.update(Event::Input('t'));
        state.update(Event::Input('e'));
        state.update(Event::Input('s'));
        state.update(Event::Input('t'));
        assert_eq!(state.input, "test");
        assert_eq!(state.cursor_pos, 4);
        state.update(Event::Submit);
        assert!(state.input.is_empty());
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn insert_char_at_middle_position() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('c'));
        state.update(Event::CursorLeft);
        state.update(Event::Input('b'));
        assert_eq!(state.input, "abc");
        assert_eq!(state.cursor_pos, 2);
    }
}
