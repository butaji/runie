//! Tests for grapheme-aware cursor movement

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::Event;

    #[test]
    fn cursor_moves_by_grapheme_not_byte() {
        let mut state = AppState::default();
        // "é" is 2 bytes but 1 grapheme
        state.update(crate::Event::Input('é'));
        assert_eq!(state.input.input, "é");
        assert_eq!(state.input.cursor_pos, 2); // byte position after 'é'
        state.update(crate::Event::CursorLeft);
        assert_eq!(state.input.cursor_pos, 0); // should go to start, not 1
    }

    #[test]
    fn cursor_right_over_multi_byte_char() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('é'));
        state.update(crate::Event::Input('a'));
        state.input.cursor_pos = 0;
        state.update(crate::Event::CursorRight);
        assert_eq!(state.input.cursor_pos, 2); // skip full 'é' grapheme
    }

    #[test]
    fn delete_before_removes_full_grapheme() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('é'));
        state.update(crate::Event::Backspace);
        assert_eq!(state.input.input, "");
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn kill_char_removes_full_grapheme() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('é'));
        state.update(crate::Event::Input('a'));
        state.input.cursor_pos = 0;
        state.update(crate::Event::KillChar);
        assert_eq!(state.input.input, "a");
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn emoji_is_one_grapheme() {
        let mut state = AppState::default();
        // "🎉" is 4 bytes
        state.update(crate::Event::Input('🎉'));
        assert_eq!(state.input.cursor_pos, 4);
        state.update(crate::Event::CursorLeft);
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn mixed_ascii_and_unicode() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Input('é'));
        state.update(crate::Event::Input('b'));
        assert_eq!(state.input.cursor_pos, 4); // 1 + 2 + 1
        state.update(crate::Event::CursorLeft);
        assert_eq!(state.input.cursor_pos, 3); // before 'b', after 'é'
        state.update(crate::Event::CursorLeft);
        assert_eq!(state.input.cursor_pos, 1); // before 'é', after 'a'
        state.update(crate::Event::CursorLeft);
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn insert_newline_at_end() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Input('b'));
        state.update(crate::Event::Newline);
        state.update(crate::Event::Input('c'));
        assert_eq!(state.input.input, "ab\nc");
        assert_eq!(state.input.cursor_pos, 4);
    }

    #[test]
    fn insert_newline_in_middle() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Input('b'));
        state.update(crate::Event::Input('c'));
        state.input.cursor_pos = 1; // after 'a'
        state.update(crate::Event::Newline);
        assert_eq!(state.input.input, "a\nbc");
        assert_eq!(state.input.cursor_pos, 2);
    }

    #[test]
    fn multiline_input_supported() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('f'));
        state.update(crate::Event::Input('i'));
        state.update(crate::Event::Input('r'));
        state.update(crate::Event::Input('s'));
        state.update(crate::Event::Input('t'));
        state.update(crate::Event::Newline);
        state.update(crate::Event::Input('l'));
        state.update(crate::Event::Input('i'));
        state.update(crate::Event::Input('n'));
        state.update(crate::Event::Input('e'));
        assert_eq!(state.input.input, "first\nline");
    }

    #[test]
    fn backspace_at_line_start_removes_newline() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Newline);
        state.update(crate::Event::Input('b'));
        // Now at end of "a\nb" (cursor after 'b')
        assert_eq!(state.input.input, "a\nb");
        assert_eq!(state.input.cursor_pos, 3); // After 'a\n' (2 chars) + 'b' (1 char)
                                               // Move cursor back to after the newline (start of second line)
        state.update(crate::Event::CursorLeft);
        assert_eq!(state.input.cursor_pos, 2); // After 'a\n'
                                               // Backspace should remove the newline and join lines
        state.update(crate::Event::Backspace);
        assert_eq!(state.input.input, "ab");
        assert_eq!(state.input.cursor_pos, 1); // Cursor at position 1 (after removing newline and 'a')
    }

    #[test]
    fn backspace_at_first_line_start_flashes() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::CursorLeft);
        assert_eq!(state.input.cursor_pos, 0);
        state.update(crate::Event::Backspace);
        // Should flash, not delete
        assert_eq!(state.input.input, "a");
        assert!(state.input.input_flash > 0);
    }

    #[test]
    fn backspace_removes_only_first_newline() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Newline);
        state.update(crate::Event::Input('b'));
        state.update(crate::Event::Newline);
        state.update(crate::Event::Input('c'));
        assert_eq!(state.input.input, "a\nb\nc");
        // Cursor is at end (after 'c')
        // Move to start of third line
        state.input.cursor_pos = 4;
        state.update(crate::Event::Backspace);
        // Should only remove the newline before 'c', not the one before 'b'
        assert_eq!(state.input.input, "a\nbc");
    }

    #[test]
    fn bash_prefix_runs_command() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('!'));
        state.update(crate::Event::Input('e'));
        state.update(crate::Event::Input('c'));
        state.update(crate::Event::Input('h'));
        state.update(crate::Event::Input('o'));
        state.update(crate::Event::Input(' '));
        state.update(crate::Event::Input('h'));
        state.update(crate::Event::Input('e'));
        state.update(crate::Event::Input('l'));
        state.update(crate::Event::Input('l'));
        state.update(crate::Event::Input('o'));
        state.update(Event::submit());
        // Command should have run and added output
        assert!(
            state
                .session
                .messages
                .iter()
                .any(|m| m.content().contains("hello")),
            "Should have hello in output"
        );
    }

    #[test]
    fn bash_prefix_not_sent_to_agent() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('!'));
        state.update(crate::Event::Input('p'));
        state.update(crate::Event::Input('w'));
        state.update(crate::Event::Input('d'));
        state.update(Event::submit());
        // Should not add to request queue
        assert!(
            state.agent.request_queue.is_empty(),
            "Bash command should not be queued for agent"
        );
    }

    #[test]
    fn regular_submit_still_works() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('h'));
        state.update(crate::Event::Input('e'));
        state.update(crate::Event::Input('l'));
        state.update(crate::Event::Input('l'));
        state.update(crate::Event::Input('o'));
        state.update(Event::submit());
        // Should add user message and queue for agent
        assert!(
            !state.agent.request_queue.is_empty(),
            "Regular submit should queue for agent"
        );
        assert_eq!(state.session.messages.len(), 1, "Should have one message");
    }

    // === Layer 2: Event handling tests (crossterm-style event → state) ===

    #[test]
    fn backspace_key_joins_lines() {
        // Test that crate::Event::Backspace joins lines when cursor is at start of a line
        let mut state = AppState::default();
        // Build: "line1\nline2"
        state.update(crate::Event::Input('l'));
        state.update(crate::Event::Input('i'));
        state.update(crate::Event::Input('n'));
        state.update(crate::Event::Input('e'));
        state.update(crate::Event::Input('1'));
        state.update(crate::Event::Newline);
        state.update(crate::Event::Input('l'));
        state.update(crate::Event::Input('i'));
        state.update(crate::Event::Input('n'));
        state.update(crate::Event::Input('e'));
        state.update(crate::Event::Input('2'));

        // Cursor is at end (position 11)
        assert_eq!(state.input.input, "line1\nline2");
        assert_eq!(state.input.cursor_pos, 11);

        // Move cursor back to position 6 (start of "line2")
        state.input.cursor_pos = 6;

        // Press Backspace - should remove newline and join lines
        state.update(crate::Event::Backspace);
        assert_eq!(state.input.input, "line1line2");
        // Cursor should be at position 5 (after "line1")
        assert_eq!(state.input.cursor_pos, 5);
    }
}
