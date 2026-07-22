//! Tests for vim-nav bracket size and shape.

use super::helpers::{
    add_message, assert_bracket_one_cell_wide, bracket_rows, draw, enter_vim_nav, state_with_wrapped_welcome,
};
use super::*;
use runie_core::Event;

#[test]
fn vim_nav_mode_bracket_spans_post_elements() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "hello".into() }],
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    state.view.last_visible_height = 10;

    enter_vim_nav(&mut state);

    let buf = draw(&mut state, 60, 20);
    let rows = bracket_rows(&buf);
    // UserInput posts don't include adjacent spacers in the bracket range
    assert!(
        !rows.is_empty(),
        "bracket should span at least the user message row"
    );
}

#[test]
fn vim_nav_mode_bracket_around_long_system_welcome_post() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    state.session.messages.push(ChatMessage {
        role: Role::System,
        parts: vec![Part::Text {
            content: "Welcome to runie in someproject.\n\nThis project is not yet trusted. \
                  Run /trust to enable write tools, or /untrust to enforce read-only mode."
                .to_string(),
        }],
        timestamp: 0.0,
        id: "trust_welcome".to_string(),
        ..Default::default()
    });
    add_message(&mut state, Role::User, "list files", 1.0, "req.0");
    state.refresh_after_message_change();

    state.view.last_visible_height = 10;

    enter_vim_nav(&mut state);
    state.update(Event::Input('k'));
    assert_eq!(state.view.selected_post, Some(0));

    let buf = draw(&mut state, 60, 20);
    let rows = bracket_rows(&buf);
    assert!(
        !rows.is_empty(),
        "selected system welcome post should have an orange bracket"
    );
}

#[test]
fn nav_mode_bracket_matches_wrapped_post_height() {
    let _lock = crate::theme::test_lock();
    let mut state = state_with_wrapped_welcome();

    let buf = draw(&mut state, 40, 24);
    let rows = bracket_rows(&buf);
    assert!(
        rows.len() >= 2,
        "wrapped system welcome post should have a multi-row bracket, got {:?}",
        rows
    );

    let first = *rows.first().unwrap();
    let last = *rows.last().unwrap();
    assert_eq!(
        buf[(0, first)].symbol(),
        "▎",
        "first visible row of a fully visible post should use the top corner glyph"
    );
    assert_eq!(
        buf[(0, last)].symbol(),
        "▎",
        "last visible row of a fully visible post should use the bottom corner glyph"
    );

    assert_bracket_one_cell_wide(&buf, &rows);
}

#[test]
fn nav_mode_bracket_for_one_line_user_post_is_three_rows() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    add_message(&mut state, Role::User, "x", 0.0, "req.0");
    state.refresh_after_message_change();

    enter_vim_nav(&mut state);

    let buf = draw(&mut state, 40, 12);
    let rows = bracket_rows(&buf);
    assert_eq!(
        rows.len(),
        3,
        "one-line user card should have a 3-row bracket (bg-padding + content + bg-padding)"
    );
    for &y in &rows {
        assert_eq!(buf[(0, y)].symbol(), "▎");
    }
}

#[test]
fn nav_mode_bracket_for_one_line_non_user_post_is_three_rows() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    add_message(&mut state, Role::User, "hi", 0.0, "req.0");
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        parts: vec![Part::Text { content: "x".into() }],
        timestamp: 1.0,
        id: "resp.0".to_string(),
        provider: "mock".to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    enter_vim_nav(&mut state);

    let buf = draw(&mut state, 40, 12);
    let rows = bracket_rows(&buf);
    assert_eq!(
        rows.len(),
        3,
        "one-line non-user post should have a 3-row bracket (spacer + content + spacer)"
    );
    for &y in &rows {
        assert_eq!(buf[(0, y)].symbol(), "▎");
    }
}
