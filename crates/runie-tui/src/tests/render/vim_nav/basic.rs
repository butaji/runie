//! Basic vim-nav visibility tests.

use super::helpers::{
    accent, add_message, draw, enter_vim_nav_and_select_top, state_with_user_agent_pairs,
};
use super::*;

#[test]
fn vim_nav_mode_shows_orange_bracket_around_selected_post() {
    let _lock = crate::theme::test_lock();
    let mut state = state_with_user_agent_pairs(10);
    enter_vim_nav_and_select_top(&mut state);

    let buf = draw(&mut state, 60, 20);
    let accent = accent();
    let mut found_line = false;
    for y in 0..buf.area().height {
        let cell = &buf[(0, y)];
        let is_bracket = cell.symbol() == "▎"
            || cell.symbol() == "╰"
            || cell.symbol() == "╭"
            || cell.symbol() == "├";
        if is_bracket && cell.style().fg == Some(accent) {
            found_line = true;
            let next = &buf[(1, y)];
            assert!(
                next.symbol() != "▎",
                "orange bracket must stay in the first cell only"
            );
        }
    }
    assert!(
        found_line,
        "vim nav mode should render an orange bracket around the selected post"
    );
}

#[test]
fn vim_nav_mode_bracket_absent_when_not_in_nav_mode() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    add_message(&mut state, Role::User, "hello", 0.0, "req.0");
    state.messages_changed();

    let buf = draw(&mut state, 60, 20);
    let accent = accent();
    let has_bracket = (0..buf.area().height).any(|y| {
        let cell = &buf[(0, y)];
        let is_bracket = cell.symbol() == "▎"
            || cell.symbol() == "╰"
            || cell.symbol() == "╭"
            || cell.symbol() == "├";
        is_bracket && cell.style().fg == Some(accent)
    });
    assert!(
        !has_bracket,
        "orange selection bracket should only appear in vim nav mode"
    );
}
