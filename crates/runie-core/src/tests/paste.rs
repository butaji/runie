//! Tests for bracketed paste handling

#[cfg(test)]
mod tests {
    use crate::model::AppState;

    #[test]
    fn paste_inserts_text_at_cursor() {
        let mut state = AppState::default();
        state.update(crate::Event::Paste("hello".into()));
        assert_eq!(state.input.input, "hello");
        assert_eq!(state.input.cursor_pos, 5);
    }

    #[test]
    fn paste_preserves_newlines() {
        let mut state = AppState::default();
        state.update(crate::Event::Paste("line1\nline2".into()));
        assert_eq!(
            state.input.input, "line1\nline2",
            "Pasted newlines must be preserved (multi-line input), not flattened"
        );
    }

    #[test]
    fn paste_normalizes_carriage_returns_to_newlines() {
        let mut state = AppState::default();
        state.update(crate::Event::Paste("a\r\nb\rc".into()));
        assert_eq!(
            state.input.input, "a\nb\nc",
            "CRLF/CR should normalize to LF"
        );
    }

    #[test]
    fn paste_replaces_tabs_with_spaces() {
        let mut state = AppState::default();
        state.update(crate::Event::Paste("a\tb".into()));
        assert_eq!(
            state.input.input, "a    b",
            "Tabs should be replaced with 4 spaces"
        );
    }

    #[test]
    fn paste_at_middle_position() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('x'));
        state.update(crate::Event::Input('z'));
        state.update(crate::Event::CursorLeft);
        state.update(crate::Event::Paste("y".into()));
        assert_eq!(state.input.input, "xyz");
        assert_eq!(state.input.cursor_pos, 2);
    }

    #[test]
    fn paste_with_existing_input() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Paste("bc".into()));
        assert_eq!(state.input.input, "abc");
        assert_eq!(state.input.cursor_pos, 3);
    }
}
