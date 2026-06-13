//! Scroll Event Handler

use crate::model::AppState;
use crate::Event;

const PAGE_SIZE: usize = 5;

pub fn scroll_event(state: &mut AppState, event: Event) {
    match event {
        Event::ScrollUp => scroll_up(state),
        Event::ScrollDown => scroll_down(state),
        Event::PageUp => page_up(state),
        Event::PageDown => page_down(state),
        Event::GoToTop => go_to_top(state),
        Event::GoToBottom => go_to_bottom(state),
        _ => {}
    }
}

/// Jump to the previous post going UP (older) in the feed.
/// Vim nav mode `k` and `ArrowUp` use this.
///
/// `view.selected_post` tracks the selection independently of scroll.
/// Each press moves the selection one post older and scrolls so the
/// selected post is at the top of the viewport when possible (near the
/// bottom it stays visible at the bottom instead).
pub fn element_jump_up(state: &mut AppState) {
    if state.view.posts.is_empty() {
        return;
    }
    let selected = state
        .view
        .selected_post
        .unwrap_or_else(|| current_top_post(state));
    if selected == 0 {
        state.input.input_flash = 3;
        scroll_to_post(state, 0);
        return;
    }
    let target = selected - 1;
    state.view.selected_post = Some(target);
    scroll_to_post(state, target);
}

/// Jump to the next post going DOWN (newer) in the feed.
/// Vim nav mode `j` and `ArrowDown` use this.
pub fn element_jump_down(state: &mut AppState) {
    if state.view.posts.is_empty() {
        return;
    }
    let selected = state
        .view
        .selected_post
        .unwrap_or_else(|| current_top_post(state));
    let last = state.view.posts.len().saturating_sub(1);
    if selected >= last {
        state.view.selected_post = Some(last);
        scroll_to_post(state, last);
        return;
    }
    let target = selected + 1;
    state.view.selected_post = Some(target);
    scroll_to_post(state, target);
}

/// Scroll so the given post is visible. The post's first element is
/// placed at the top of the viewport when it fits; otherwise the
/// viewport is snapped to the bottom so the post remains visible near
/// the end of the feed.
fn scroll_to_post(state: &mut AppState, post_index: usize) {
    let visible = state.view.last_visible_height.max(3) as usize;
    let total = state.view.total_lines;
    let max_scroll = total.saturating_sub(visible);
    let cum = cumulative_line_counts(&state.view.line_counts);
    let first_element = state
        .view
        .posts
        .get(post_index)
        .map(|p| p.start)
        .unwrap_or(0);
    let target_top = if first_element == 0 {
        0
    } else {
        cum.get(first_element - 1).copied().unwrap_or(0)
    };
    state.view.scroll = max_scroll.saturating_sub(target_top);
}

/// Index of the post currently at the top of the viewport.
fn current_top_post(state: &AppState) -> usize {
    crate::snapshot::compute_current_top_element(
        &state.view.elements_cache,
        &state.view.line_counts,
        state.view.total_lines,
        state.view.scroll,
        state.view.last_visible_height,
    )
    .and_then(|elem| post_for_element(state, elem))
    .unwrap_or(0)
}

/// Find the post that contains the given element index.
fn post_for_element(state: &AppState, element_index: usize) -> Option<usize> {
    state
        .view
        .posts
        .iter()
        .find(|p| p.start <= element_index && element_index < p.end)
        .map(|p| p.index)
}

/// Returns cumulative line counts, where index i is the total number of
/// lines used by elements [0..=i]. Length equals line_counts.len().
fn cumulative_line_counts(line_counts: &[usize]) -> Vec<usize> {
    let mut cum = Vec::with_capacity(line_counts.len());
    let mut total = 0usize;
    for &c in line_counts {
        total += c;
        cum.push(total);
    }
    cum
}

fn scroll_up(state: &mut AppState) {
    if state.session.messages.is_empty() && !state.agent.turn_active {
        state.input.input_flash = 3;
    }
    state.view.scroll = state.view.scroll.saturating_add(1);
}

fn scroll_down(state: &mut AppState) {
    if state.view.scroll == 0 {
        state.input.input_flash = 3;
    }
    state.view.scroll = state.view.scroll.saturating_sub(1);
}

fn page_up(state: &mut AppState) {
    if state.session.messages.is_empty() && !state.agent.turn_active {
        state.input.input_flash = 3;
    }
    state.view.scroll = state.view.scroll.saturating_add(PAGE_SIZE);
}

fn page_down(state: &mut AppState) {
    if state.view.scroll == 0 {
        state.input.input_flash = 3;
    }
    state.view.scroll = state.view.scroll.saturating_sub(PAGE_SIZE);
}

fn go_to_top(state: &mut AppState) {
    if state.session.messages.is_empty() && !state.agent.turn_active {
        state.input.input_flash = 3;
    }
    let visible = state.view.last_visible_height.max(3) as usize;
    let max_scroll = state.view.total_lines.saturating_sub(visible);
    // Land the viewport at the very top of the feed.
    state.view.scroll = max_scroll;
    if state.vim_nav_mode {
        // `g` always selects the oldest (top) post.
        state.view.selected_post = Some(0);
    }
}

fn go_to_bottom(state: &mut AppState) {
    state.view.scroll = 0;
    if state.vim_nav_mode {
        let len = state.view.posts.len();
        state.view.selected_post = len.checked_sub(1);
    }
}
