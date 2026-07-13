//! Tests for vim-nav selection background and non-selection areas.

use super::helpers::{accent_bg, add_message, draw, state_with_selected_post};
use super::*;
use crate::tests::connect_model;
use runie_core::Event;

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
        // The selection band aligns with the user-card band: it covers
        // columns 2..width-2 and leaves a two-column feed-background margin
        // on each side. Painting to the last column would leave the
        // terminal in the deferred-wrap state, and tmux then drops the next
        // row's background attributes (verified against tmux 3.7b).
        let left_margin_clear = buf[(0, y)].style().bg != Some(bg)
            && buf[(1, y)].style().bg != Some(bg);
        let band_start = buf[(2, y)].style().bg == Some(bg);
        let band_end = buf[(width - 3, y)].style().bg == Some(bg);
        let right_margin_clear = buf[(width - 2, y)].style().bg != Some(bg)
            && buf[(width - 1, y)].style().bg != Some(bg);
        assert!(
            left_margin_clear && band_start && band_end && right_margin_clear,
            "row {y} selection band must span cols 2..width-2 with clear margins"
        );
    }
}

#[test]
fn user_post_in_feed_has_background_color() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");
    state.refresh_after_message_change();
    state.update(Event::DialogBack); // Enter vim_nav_mode

    let buf = draw(&mut state, 60, 12);
    let bg = accent_bg();
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
        "user message content should render on the selected-post accent background"
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

#[test]
fn user_message_has_bg_user_background() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");
    state.refresh_after_message_change();

    let buf = draw(&mut state, 60, 12);
    let bg = crate::theme::color_bg_user();

    // User message should have bg.user background on some cells
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            let cell = &buf[(x, y)];
            if cell.style().bg == Some(bg) {
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
        "user message should have bg.user background color"
    );
}

/// Spec for a user message card (per design):
///   -- one bg-banded line ABOVE the content (cols 2..width-2)
///   -- content line(s) with bg band
///   -- one bg-banded line BELOW the content
///   -- one normal (feed-bg) margin line after that
/// The band keeps a two-column feed-background margin on each side.
/// The card structure must hold when the user message is followed by an agent
/// response: bg-above, content, bg-below, then a normal margin line before the
/// next post's content.
#[test]
fn user_card_followed_by_agent_keeps_margin_line() {
    let _lock = crate::theme::test_lock();
    let bg = crate::theme::color_bg_user();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello there", 0.0, "req.0");
    add_message(&mut state, Role::Assistant, "agent reply", 1.0, "resp.0");
    state.refresh_after_message_change();

    let buf = draw(&mut state, 120, 30);
    let w = buf.area().width;
    let row_bg = |y: u16| (2..w - 2).all(|x| buf[(x, y)].style().bg == Some(bg));
    let row_text = |y: u16| {
        (0..w)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect::<String>()
    };

    // Find the user content row (has the ❯ glyph).
    let content_row = (0..buf.area().height)
        .find(|&y| row_text(y).contains('❯'))
        .expect("user content row not found");
    // Find the agent content row (plain answer text, no leading glyph).
    let agent_row = (0..buf.area().height)
        .find(|&y| row_text(y).contains("agent reply"))
        .expect("agent content row not found");

    assert!(row_bg(content_row - 1), "bg line above user content");
    assert!(row_bg(content_row), "user content has bg");
    assert!(row_bg(content_row + 1), "bg line below user content");
    assert!(
        !row_bg(content_row + 2),
        "margin line after the card must be the normal feed background"
    );
    // The agent post must come after the margin line, not directly under the bg.
    assert!(
        agent_row > content_row + 2,
        "agent content (row {agent_row}) must be separated from the card by a normal margin line"
    );
    assert!(!row_bg(agent_row), "agent content must not carry bg.user");
}

#[test]
fn user_message_card_has_bg_padding_and_margin() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");
    state.refresh_after_message_change();

    let buf = draw(&mut state, 60, 12);
    let bg = crate::theme::color_bg_user();
    let width = buf.area().width;

    // Locate the content row (the one containing the user text "hello").
    let content_row = (0..buf.area().height)
        .find(|&y| {
            (0..width).any(|x| {
                buf[(x, y)].symbol() == "h" && buf[(x, y)].style().bg == Some(bg)
            })
        })
        .expect("user content row with bg not found");

    let row_bg = |y: u16| -> bool {
        (2..width - 2).all(|x| buf[(x, y)].style().bg == Some(bg))
    };

    assert!(
        content_row >= 1,
        "content row must have a bg line above it (found at row {content_row})"
    );
    assert!(
        row_bg(content_row - 1),
        "line above user content (row {}) must be a bg-banded line",
        content_row - 1
    );
    assert!(
        row_bg(content_row),
        "user content row {content_row} must have the bg band"
    );
    assert!(
        row_bg(content_row + 1),
        "line below user content (row {}) must be a bg-banded line",
        content_row + 1
    );
    assert!(
        !row_bg(content_row + 2),
        "margin line after the card (row {}) must use the normal feed background, not bg.user",
        content_row + 2
    );
}

/// The card structure must hold even when the user message is the very first
/// post in the feed (no preceding element to lend a bg line).
#[test]
fn first_user_message_card_has_bg_line_above() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");
    state.refresh_after_message_change();

    let buf = draw(&mut state, 60, 12);
    let bg = crate::theme::color_bg_user();
    let width = buf.area().width;

    let content_row = (0..buf.area().height)
        .find(|&y| (0..width).any(|x| buf[(x, y)].symbol() == "h"))
        .expect("user content row not found");

    assert!(
        content_row >= 1,
        "first user message must still have a bg line above its content"
    );
    let above_is_bg =
        (2..width - 2).all(|x| buf[(x, content_row - 1)].style().bg == Some(bg));
    assert!(
        above_is_bg,
        "row above the first user message (row {}) must be a bg-banded line",
        content_row - 1
    );
}

#[test]
fn user_message_background_spans_band_with_two_column_margins() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    add_message(&mut state, Role::User, "hello", 0.0, "req.0");
    state.refresh_after_message_change();

    let buf = draw(&mut state, 60, 12);
    let bg = crate::theme::color_bg_user();
    let width = buf.area().width;

    // Find banded rows and check the band spans cols 2..width-2 with the
    // two-column margins on each side staying on the feed background.
    let mut banded = 0;
    for y in 0..buf.area().height {
        if buf[(2, y)].style().bg == Some(bg) {
            banded += 1;
            for x in 0..2 {
                assert_ne!(
                    buf[(x, y)].style().bg,
                    Some(bg),
                    "left margin col {x} must stay on the feed background at row {y}"
                );
            }
            for x in 2..width - 2 {
                assert_eq!(
                    buf[(x, y)].style().bg,
                    Some(bg),
                    "band col {x} must carry bg.user at row {y}"
                );
            }
            for x in width - 2..width {
                assert_ne!(
                    buf[(x, y)].style().bg,
                    Some(bg),
                    "right margin col {x} must stay on the feed background at row {y}"
                );
            }
        }
    }
    assert!(banded >= 3, "user card should render bg band rows");
}
