//! Scroll-related message handlers.

use crate::components::MessageItem;
use super::ChatCmd;
use crate::tui::state::{AppState, Msg};

/// A "page" in the scroll model is one viewport-height worth of messages.
/// 20 is the conventional default; tests rely on this to reach the end of
/// long feeds in a reasonable number of PageDown presses.
const PAGE_SIZE: i32 = 20;
const HALF_PAGE_SIZE: i32 = PAGE_SIZE / 2;

pub fn handle_scroll_msg(state: &mut AppState, msg: &crate::tui::state::Msg) -> Vec<ChatCmd> {
    
    if let Some(result) = handle_scroll_direction(msg, state) {
        return result;
    }
    if let Some(result) = handle_scroll_position(msg, state) {
        return result;
    }
    vec![]
}

fn handle_scroll_direction(msg: &Msg, state: &mut AppState) -> Option<Vec<ChatCmd>> {
    match msg {
        Msg::ScrollUp => Some(scroll_by(state, 1)),
        Msg::ScrollDown => Some(scroll_by(state, -1)),
        Msg::ScrollPageUp => Some(scroll_by(state, PAGE_SIZE)),
        Msg::ScrollPageDown => Some(scroll_by(state, -PAGE_SIZE)),
        Msg::ScrollHalfPageUp => Some(scroll_by(state, HALF_PAGE_SIZE)),
        Msg::ScrollHalfPageDown => Some(scroll_by(state, -HALF_PAGE_SIZE)),
        _ => None,
    }
}

fn handle_scroll_position(msg: &Msg, state: &mut AppState) -> Option<Vec<ChatCmd>> {
    match msg {
        Msg::ScrollToTop => Some(scroll_to_top(state)),
        Msg::ScrollToBottom => Some(scroll_to_bottom(state)),
        Msg::ScrollToPrevUserTurn => Some(scroll_to_prev_user_turn(state)),
        Msg::ScrollToNextUserTurn => Some(scroll_to_next_user_turn(state)),
        _ => None,
    }
}

fn scroll_by(state: &mut AppState, delta: i32) -> Vec<ChatCmd> {
    let page = delta.unsigned_abs() as usize;
    let new_offset = if delta > 0 {
        state.scroll.feed_offset.saturating_sub(page)
    } else {
        // saturating_add guards against usize overflow when feed_offset has
        // been set to an extreme value (e.g. usize::MAX from a test) — the
        // .min below then clamps to the last valid message index.
        state.scroll
            .feed_offset
            .saturating_add(page)
            .min(state.messages.len().saturating_sub(1))
    };
    state.scroll.feed_offset = new_offset;
    state.scroll.user_scrolled_up = new_offset > 0;
    vec![]
}

fn scroll_to_top(state: &mut AppState) -> Vec<ChatCmd> {
    state.scroll.feed_offset = 0;
    state.scroll.user_scrolled_up = false;
    vec![]
}

fn scroll_to_bottom(state: &mut AppState) -> Vec<ChatCmd> {
    state.scroll.feed_offset = state.messages.len().saturating_sub(1);
    state.scroll.user_scrolled_up = true;
    vec![]
}

fn scroll_to_prev_user_turn(state: &mut AppState) -> Vec<ChatCmd> {
    // Find the previous user message before current offset
    let current_offset = state.scroll.feed_offset;
    let mut new_offset = current_offset;

    for i in (0..current_offset).rev() {
        if matches!(state.messages.get(i), Some(MessageItem::User { .. })) {
            new_offset = i;
            break;
        }
        if i == 0 {
            new_offset = 0;
        }
    }
    state.scroll.feed_offset = new_offset;
    state.scroll.user_scrolled_up = new_offset > 0;
    vec![]
}

fn scroll_to_next_user_turn(state: &mut AppState) -> Vec<ChatCmd> {
    // Find the next user message after current offset
    let current_offset = state.scroll.feed_offset;
    let mut new_offset = current_offset;

    for i in (current_offset + 1)..state.messages.len() {
        if matches!(state.messages.get(i), Some(MessageItem::User { .. })) {
            new_offset = i;
            break;
        }
        if i == state.messages.len() - 1 {
            new_offset = state.messages.len().saturating_sub(1);
        }
    }
    state.scroll.feed_offset = new_offset;
    state.scroll.user_scrolled_up = new_offset > 0;
    vec![]
}
