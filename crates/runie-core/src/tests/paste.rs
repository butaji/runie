//! Tests for bracketed paste handling

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::event::Event;

    #[test]
    fn paste_inserts_text_at_cursor() {
        let mut state = AppState::default();
        state.update(Event::Paste("hello".to_string()));
        assert_eq!(state.input.input, "hello");
        assert_eq!(state.input.cursor_pos, 5);
    }

    #[test]
    fn paste_strips_newlines() {
        let mut state = AppState::default();
        state.update(Event::Paste("line1\nline2".to_string()));
        assert_eq!(state.input.input, "line1line2", "Newlines should be stripped from paste");
    }

    #[test]
    fn paste_strips_carriage_returns() {
        let mut state = AppState::default();
        state.update(Event::Paste("a\r\nb".to_string()));
        assert_eq!(state.input.input, "ab", "CRLF should be stripped");
    }

    #[test]
    fn paste_replaces_tabs_with_spaces() {
        let mut state = AppState::default();
        state.update(Event::Paste("a\tb".to_string()));
        assert_eq!(state.input.input, "a    b", "Tabs should be replaced with 4 spaces");
    }

    #[test]
    fn paste_at_middle_position() {
        let mut state = AppState::default();
        state.update(Event::Input('x'));
        state.update(Event::Input('z'));
        state.update(Event::CursorLeft);
        state.update(Event::Paste("y".to_string()));
        assert_eq!(state.input.input, "xyz");
        assert_eq!(state.input.cursor_pos, 2);
    }

    #[test]
    fn paste_with_existing_input() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Paste("bc".to_string()));
        assert_eq!(state.input.input, "abc");
        assert_eq!(state.input.cursor_pos, 3);
    }
}
