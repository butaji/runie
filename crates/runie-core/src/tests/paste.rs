//! Tests for bracketed paste handling


#[cfg(test)]
mod tests {
    use crate::event::InputEvent;
    use crate::model::AppState;

    #[test]
    fn paste_inserts_text_at_cursor() {
        let mut state = AppState::default();
        state.update(InputEvent::Paste("hello".into()));
        assert_eq!(state.input.input, "hello");
        assert_eq!(state.input.cursor_pos, 5);
    }

    #[test]
    fn paste_strips_newlines() {
        let mut state = AppState::default();
        state.update(InputEvent::Paste("line1\nline2".into()));
        assert_eq!(
            state.input.input, "line1line2",
            "Newlines should be stripped from paste"
        );
    }

    #[test]
    fn paste_strips_carriage_returns() {
        let mut state = AppState::default();
        state.update(InputEvent::Paste("a\r\nb".into()));
        assert_eq!(state.input.input, "ab", "CRLF should be stripped");
    }

    #[test]
    fn paste_replaces_tabs_with_spaces() {
        let mut state = AppState::default();
        state.update(InputEvent::Paste("a\tb".into()));
        assert_eq!(
            state.input.input, "a    b",
            "Tabs should be replaced with 4 spaces"
        );
    }

    #[test]
    fn paste_at_middle_position() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('x'));
        state.update(InputEvent::Input('z'));
        state.update(InputEvent::CursorLeft);
        state.update(InputEvent::Paste("y".into()));
        assert_eq!(state.input.input, "xyz");
        assert_eq!(state.input.cursor_pos, 2);
    }

    #[test]
    fn paste_with_existing_input() {
        let mut state = AppState::default();
        state.update(InputEvent::Input('a'));
        state.update(InputEvent::Paste("bc".into()));
        assert_eq!(state.input.input, "abc");
        assert_eq!(state.input.cursor_pos, 3);
    }
}
