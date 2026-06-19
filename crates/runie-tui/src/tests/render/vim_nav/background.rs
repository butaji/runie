//! Tests for vim-nav selection background and non-selection areas.

use super::helpers::{accent_bg, add_message, draw, state_with_selected_post};
use crate::tests::connect_model;
use runie_core::{AppState, Role};

#[test]
fn nav_mode_selected_post_has_accent_background() {
    let _lock = crate::theme::test_lock();
    let mut state = state_with_selected_post();

    let buf = draw(&mut state, 40, 12);
    let bg = accent_bg();
    let line_rows: Vec<u16> = (0..buf.area().height)
        .filter(|&y| buf[(0, y)].symbol() == "▎")
        .collect();
    assert!(
        !line_rows.is_empty(),
        "selected post should have a visible left line"
    );
    let width = buf.area().width;
    for y in line_rows {
        let left_bg = buf[(0, y)].style().bg == Some(bg);
        let right_bg = buf[(width - 1, y)].style().bg == Some(bg);
        assert!(
            left_bg && right_bg,
            "row {y} selection background must cover the whole line, including margins"
        );
    }
}

#[test]
fn user_post_in_feed_has_background_color() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");
    state.messages_changed();
    state.ensure_fresh();

    let buf = draw(&mut state, 60, 12);
    let bg = crate::theme::color_user_bg();
    assert_ne!(
        bg,
        ratatui::style::Color::Reset,
        "user post background must be a non-default color"
    );

    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            if buf[(x, y)].symbol() == "h" && buf[(x, y)].style().bg == Some(bg) {
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }
    assert!(
        found,
        "user message content should render on the user bubble background"
    );
}

#[test]
fn input_box_chevron_has_no_accent_background() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);

    let buf = draw(&mut state.clone(), 60, 12);
    let bg = accent_bg();
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            if buf[(x, y)].symbol() == "❯" {
                assert_ne!(
                    buf[(x, y)].style().bg,
                    Some(bg),
                    "input box chevron must not carry the selected-post accent background"
                );
                found = true;
            }
        }
    }
    assert!(found, "input chevron not found");
}
