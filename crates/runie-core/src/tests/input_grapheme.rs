//! Tests for grapheme-aware cursor movement

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::event::Event;

    #[test]
    fn cursor_moves_by_grapheme_not_byte() {
        let mut state = AppState::default();
        // "é" is 2 bytes but 1 grapheme
        state.update(Event::Input('é'));
        assert_eq!(state.input, "é");
        assert_eq!(state.cursor_pos, 2); // byte position after 'é'
        state.update(Event::CursorLeft);
        assert_eq!(state.cursor_pos, 0); // should go to start, not 1
    }

    #[test]
    fn cursor_right_over_multi_byte_char() {
        let mut state = AppState::default();
        state.update(Event::Input('é'));
        state.update(Event::Input('a'));
        state.cursor_pos = 0;
        state.update(Event::CursorRight);
        assert_eq!(state.cursor_pos, 2); // skip full 'é' grapheme
    }

    #[test]
    fn delete_before_removes_full_grapheme() {
        let mut state = AppState::default();
        state.update(Event::Input('é'));
        state.update(Event::Backspace);
        assert_eq!(state.input, "");
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn kill_char_removes_full_grapheme() {
        let mut state = AppState::default();
        state.update(Event::Input('é'));
        state.update(Event::Input('a'));
        state.cursor_pos = 0;
        state.update(Event::KillChar);
        assert_eq!(state.input, "a");
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn emoji_is_one_grapheme() {
        let mut state = AppState::default();
        // "🎉" is 4 bytes
        state.update(Event::Input('🎉'));
        assert_eq!(state.cursor_pos, 4);
        state.update(Event::CursorLeft);
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn mixed_ascii_and_unicode() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('é'));
        state.update(Event::Input('b'));
        assert_eq!(state.cursor_pos, 4); // 1 + 2 + 1
        state.update(Event::CursorLeft);
        assert_eq!(state.cursor_pos, 3); // before 'b', after 'é'
        state.update(Event::CursorLeft);
        assert_eq!(state.cursor_pos, 1); // before 'é', after 'a'
        state.update(Event::CursorLeft);
        assert_eq!(state.cursor_pos, 0);
    }
}
