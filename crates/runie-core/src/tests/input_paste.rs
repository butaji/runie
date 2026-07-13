//! Layer 2 tests for bracketed paste of long text.

use crate::model::AppState;

#[test]
fn paste_long_text_inserts_full_content() {
    let mut state = AppState::default();
    let text = "a".repeat(5000);
    state.update(crate::Event::Paste(text.clone()));
    assert_eq!(state.input.input, text);
    assert_eq!(state.input.cursor_pos, 5000);
}

#[test]
fn paste_at_end_appends_and_moves_cursor() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('x'));
    state.update(crate::Event::Paste("yz".into()));
    assert_eq!(state.input.input, "xyz");
    assert_eq!(state.input.cursor_pos, 3);
}

#[test]
fn paste_replaces_newlines_and_carriage_returns_with_spaces() {
    let mut state = AppState::default();
    state.update(crate::Event::Paste("line1\r\nline2\nline3".into()));
    assert_eq!(state.input.input, "line1 line2 line3");
}

#[test]
fn paste_replaces_tabs_with_spaces() {
    let mut state = AppState::default();
    state.update(crate::Event::Paste("a\tb".into()));
    assert_eq!(state.input.input, "a    b");
    assert_eq!(state.input.cursor_pos, 6);
}
