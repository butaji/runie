//! Tests for vim nav mode j/k post-level jumping.
//!
//! In nav mode `j` moves DOWN (toward the newest, bottom of the feed)
//! and `k` moves UP (toward the oldest, top of the feed). Each press
//! jumps by one element (thinking block, tool call, user message,
//! assistant response). This is element-level navigation, distinct
//! from line-level scrolling.

use runie_core::Event;
use runie_core::model::{AppState, ChatMessage, Role};

fn state_with_vim_and_messages() -> AppState {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.view.last_content_width = 80;
    for i in 0..20 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            content: format!("user {}", i),
            timestamp: i as f64,
            id: format!("req.{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: format!("assistant {}", i),
            timestamp: i as f64 + 0.5,
            id: format!("resp.{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state
}

fn enter_nav(state: &mut AppState) {
    state.update(Event::DialogBack);
    assert!(state.view.vim_nav_mode);
}

#[test]
fn k_in_nav_mode_jumps_up_at_least_one_post() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('g')); // top
    let top = state.view.scroll;
    state.update(Event::Input('j')); // down a bit
    let mid = state.view.scroll;
    assert!(mid < top, "j should move toward bottom (newer)");

    state.update(Event::Input('k')); // k = up (older)
    let after_k = state.view.scroll;
    assert!(
        after_k > mid,
        "k should move up (older): mid={mid} after_k={after_k}"
    );
    let delta = after_k - mid;
    assert!(delta >= 2, "k should jump at least 2 lines, got {delta}");
}

#[test]
fn j_in_nav_mode_jumps_down_at_least_one_post() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('g')); // to top (nav-mode motion)
    let top = state.view.scroll;
    assert!(top > 0);

    state.update(Event::Input('j')); // j = down (newer)
    let after_j = state.view.scroll;
    assert!(
        after_j < top,
        "j should move down (newer): top={top} after_j={after_j}"
    );
    let delta = top - after_j;
    assert!(delta >= 2, "j should jump at least 2 lines, got {delta}");
}

#[test]
fn j_then_k_round_trip() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('g')); // top
    let top = state.view.scroll;
    state.update(Event::Input('j')); // down
    let mid = state.view.scroll;
    assert!(mid < top);

    state.update(Event::Input('k')); // up
    let after_k = state.view.scroll;
    assert!(after_k > mid, "k should move back up");

    state.update(Event::Input('j')); // down again
    let after_j = state.view.scroll;
    assert!(after_j < after_k, "j should move back down toward newer");
}

#[test]
fn j_at_bottom_exits_nav_mode() {
    // At the lowest post, j exits nav mode (the next focus is the
    // input box). This replaces the old "flash at bottom" behavior.
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    assert_eq!(state.view.scroll, 0);
    state.update(Event::Input('j'));
    assert!(!state.view.vim_nav_mode, "j at bottom should exit nav mode");
}

#[test]
fn k_at_top_flashes_and_does_not_overshoot() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('g'));
    let max = state
        .view
        .total_lines
        .saturating_sub(state.view.last_visible_height as usize);
    assert_eq!(state.view.scroll, max);
    let prev_flash = state.input.input_flash;
    state.update(Event::Input('k'));
    assert!(state.view.scroll >= max, "k at top should not overshoot");
    assert!(
        state.input.input_flash > prev_flash,
        "k at top should flash"
    );
}

#[test]
fn g_goes_to_top_of_feed() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);

    state.update(Event::Input('g'));
    assert!(state.view.scroll > 0, "g should scroll away from bottom");
    let max = state
        .view
        .total_lines
        .saturating_sub(state.view.last_visible_height as usize);
    assert_eq!(
        state.view.scroll, max,
        "g should land at max_scroll (top of viewport at file top)"
    );
}

#[test]
#[allow(non_snake_case)]
fn capital_G_goes_to_bottom() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('g'));
    assert!(state.view.scroll > 0);
    state.update(Event::Input('G'));
    assert_eq!(state.view.scroll, 0);
}

#[test]
fn line_j_k_when_not_in_nav_mode() {
    // Outside nav mode (vim_mode on, empty input), j/k should still
    // scroll by one line each (legacy behavior).
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    assert!(!state.view.vim_nav_mode);
    state.update(Event::Input('j'));
    assert_eq!(state.view.scroll, 1, "line-level j: +1");
    state.update(Event::Input('k'));
    assert_eq!(state.view.scroll, 0, "line-level k: -1");
}

// =========================================================================
// Arrow Up / Arrow Down in nav mode follow the same j-down / k-up
// direction: ArrowDown moves toward newer, ArrowUp moves toward older.
// =========================================================================

#[test]
fn arrow_down_in_nav_mode_moves_toward_newer() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('g'));
    let top = state.view.scroll;
    state.update(Event::HistoryNext);
    let after = state.view.scroll;
    assert!(after < top, "ArrowDown must move toward newer");
    assert!(top - after >= 2, "ArrowDown must jump at least 2 lines");
}

#[test]
fn arrow_up_in_nav_mode_moves_toward_older() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('g'));
    let _top = state.view.scroll;
    state.update(Event::Input('j'));
    let mid = state.view.scroll;
    state.update(Event::HistoryPrev);
    let after = state.view.scroll;
    assert!(after > mid, "ArrowUp must move toward older");
    assert!(after - mid >= 2, "ArrowUp must jump at least 2 lines");
}

#[test]
fn arrow_down_at_bottom_exits_nav_mode() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::HistoryNext);
    assert!(
        !state.view.vim_nav_mode,
        "ArrowDown at bottom should exit nav mode"
    );
}

#[test]
fn arrow_up_at_top_flashes_and_does_not_overshoot() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    state.update(Event::Input('g'));
    let max = state
        .view
        .total_lines
        .saturating_sub(state.view.last_visible_height as usize);
    assert_eq!(state.view.scroll, max);
    let prev_flash = state.input.input_flash;
    state.update(Event::HistoryPrev);
    assert!(
        state.view.scroll >= max,
        "ArrowUp at top should not overshoot"
    );
    assert!(
        state.input.input_flash > prev_flash,
        "ArrowUp at top should flash"
    );
}

#[test]
fn arrow_keys_outside_nav_mode_still_navigate_input_history() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    assert!(!state.view.vim_nav_mode);
    let before = state.view.scroll;
    state.update(Event::HistoryPrev);
    assert_eq!(
        state.view.scroll, before,
        "ArrowUp outside nav must not scroll"
    );
}

// =========================================================================
// Every post must be selectable: j/ArrowDown and k/ArrowUp must move
// the selection one element at a time, without skipping.
// =========================================================================

fn selected_post(state: &mut AppState) -> Option<usize> {
    state.view.selected_post
}

fn collect_selections<F>(mut state: AppState, mut step: F) -> Vec<usize>
where
    F: FnMut(&mut AppState),
{
    let mut visited = Vec::new();
    if let Some(sel) = selected_post(&mut state) {
        visited.push(sel);
    }
    loop {
        step(&mut state);
        if !state.view.vim_nav_mode {
            // Motion at the boundary exits nav mode; the selection stays
            // on the final post, so record it before re-arming.
            if let Some(sel) = selected_post(&mut state) {
                if Some(sel) != visited.last().copied() {
                    visited.push(sel);
                }
            }
            state.view.vim_nav_mode = true;
            break;
        }
        let sel = selected_post(&mut state).expect("should have a selection");
        if Some(sel) == visited.last().copied() {
            break;
        }
        visited.push(sel);
    }
    visited
}

#[test]
fn j_visits_every_post_from_top_to_bottom() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('g'));

    let last_index = state.view.posts.len().saturating_sub(1);
    let visited = collect_selections(state, |s| s.update(Event::Input('j')));
    assert_eq!(visited.first().copied(), Some(0));
    assert_eq!(
        visited.last().copied(),
        Some(last_index),
        "j should reach the last post"
    );
    for window in visited.windows(2) {
        assert_eq!(window[0] + 1, window[1], "j skipped a post");
    }
}

#[test]
fn k_visits_every_post_from_bottom_to_top() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('G')); // start at the last post

    let last_index = state.view.posts.len().saturating_sub(1);
    let visited = collect_selections(state, |s| s.update(Event::Input('k')));
    assert_eq!(visited.first().copied(), Some(last_index));
    assert_eq!(visited.last().copied(), Some(0));
    for window in visited.windows(2) {
        assert_eq!(window[0], window[1] + 1, "k skipped a post");
    }
}

#[test]
fn arrow_down_visits_every_post_from_top_to_bottom() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('g'));

    let last_index = state.view.posts.len().saturating_sub(1);
    let visited = collect_selections(state, |s| s.update(Event::HistoryNext));
    assert_eq!(visited.first().copied(), Some(0));
    assert_eq!(
        visited.last().copied(),
        Some(last_index),
        "ArrowDown should reach the last post"
    );
    for window in visited.windows(2) {
        assert_eq!(window[0] + 1, window[1], "ArrowDown skipped a post");
    }
}

#[test]
fn arrow_up_visits_every_post_from_bottom_to_top() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(Event::Input('G')); // start at the last post

    let last_index = state.view.posts.len().saturating_sub(1);
    let visited = collect_selections(state, |s| s.update(Event::HistoryPrev));
    assert_eq!(visited.first().copied(), Some(last_index));
    assert_eq!(visited.last().copied(), Some(0));
    for window in visited.windows(2) {
        assert_eq!(window[0], window[1] + 1, "ArrowUp skipped a post");
    }
}

#[test]
fn j_and_arrow_down_visit_identical_posts() {
    let mut state_j = state_with_vim_and_messages();
    state_j.view.last_visible_height = 10;
    enter_nav(&mut state_j);
    state_j.update(Event::Input('g'));

    let mut state_arrow = state_with_vim_and_messages();
    state_arrow.view.last_visible_height = 10;
    enter_nav(&mut state_arrow);
    state_arrow.update(Event::Input('g'));

    for _ in 0..state_j.view.posts.len() {
        assert_eq!(
            selected_post(&mut state_j),
            selected_post(&mut state_arrow),
            "j and ArrowDown must select the same post"
        );
        state_j.update(Event::Input('j'));
        state_arrow.update(Event::HistoryNext);
    }
}

fn state_with_welcome_post() -> AppState {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.view.last_content_width = 80;
    state.session.messages.push(ChatMessage {
        role: Role::System,
        content: "Welcome to runie in someproject.\n\nThis project is not yet trusted. \
                  Run /trust to enable write tools, or /untrust to enforce read-only mode."
            .to_string(),
        timestamp: 0.0,
        id: "trust_welcome".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "list files".to_string(),
        timestamp: 1.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();
    state.view.last_visible_height = 10;
    state
}

#[test]
fn long_system_welcome_post_is_selectable() {
    let mut state = state_with_welcome_post();

    enter_nav(&mut state);
    assert_eq!(state.view.selected_post, Some(1));

    state.update(Event::Input('k'));
    assert_eq!(
        state.view.selected_post,
        Some(0),
        "k should move selection to the long system welcome post"
    );

    state.update(Event::Input('j'));
    assert_eq!(
        state.view.selected_post,
        Some(1),
        "j should move selection back down from the system welcome post"
    );
}

#[test]
fn collapsed_and_expanded_posts_have_same_post_count() {
    let mut collapsed = state_with_vim_and_messages();
    collapsed.view.last_visible_height = 10;
    collapsed.view.all_collapsed = true;
    collapsed.messages_changed();
    collapsed.ensure_fresh();

    let mut expanded = state_with_vim_and_messages();
    expanded.view.last_visible_height = 10;
    expanded.view.all_collapsed = false;
    expanded.messages_changed();
    expanded.ensure_fresh();

    assert_eq!(
        collapsed.view.posts.len(),
        expanded.view.posts.len(),
        "collapse state must not change the number of navigable posts"
    );
}

#[test]
fn k_and_arrow_up_visit_identical_posts() {
    let mut state_k = state_with_vim_and_messages();
    state_k.view.last_visible_height = 10;
    enter_nav(&mut state_k);

    let mut state_arrow = state_with_vim_and_messages();
    state_arrow.view.last_visible_height = 10;
    enter_nav(&mut state_arrow);

    for _ in 0..state_k.view.posts.len() {
        assert_eq!(
            selected_post(&mut state_k),
            selected_post(&mut state_arrow),
            "k and ArrowUp must select the same post"
        );
        state_k.update(Event::Input('k'));
        state_arrow.update(Event::HistoryPrev);
    }
}
