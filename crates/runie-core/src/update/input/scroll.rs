//! Scroll event handler (from input_scroll.rs).

use crate::model::AppState;

pub const PAGE_SIZE: usize = super::nav::PAGE_SIZE;

pub fn scroll_event(state: &mut AppState, event: crate::Event) {
    match event {
        crate::Event::Up => scroll_up(state),
        crate::Event::Down => scroll_down(state),
        crate::Event::PageUp => page_up(state),
        crate::Event::PageDown => page_down(state),
        crate::Event::GoToTop => go_to_top(state),
        crate::Event::GoToBottom => go_to_bottom(state),
        // intentionally ignored: other scroll events fall through
        _ => {}
    }
}

pub fn element_jump_up(state: &mut AppState) {
    let snap = state.snapshot();
    if snap.posts.is_empty() {
        return;
    }
    let selected = state
        .view
        .selected_post
        .unwrap_or_else(|| current_top_post(&snap));
    if selected == 0 {
        state.input_mut().input_flash = 3;
        scroll_to_post(state, &snap, 0);
        return;
    }
    let target = selected - 1;
    state.view_mut().selected_post = Some(target);
    scroll_to_post(state, &snap, target);
}

pub fn element_jump_down(state: &mut AppState) {
    let snap = state.snapshot();
    if snap.posts.is_empty() {
        return;
    }
    let selected = state
        .view
        .selected_post
        .unwrap_or_else(|| current_top_post(&snap));
    let last = snap.posts.len().saturating_sub(1);
    if selected >= last {
        state.view_mut().selected_post = Some(last);
        scroll_to_post(state, &snap, last);
        return;
    }
    let target = selected + 1;
    state.view_mut().selected_post = Some(target);
    scroll_to_post(state, &snap, target);
}

fn scroll_to_post(state: &mut AppState, snap: &crate::Snapshot, post_index: usize) {
    let visible = state.view_mut().last_visible_height.max(3) as usize;
    let total = snap.total_lines;
    let max_scroll = total.saturating_sub(visible);
    let cum = cumulative_line_counts(&snap.line_counts);
    let first_element = snap.posts.get(post_index).map(|p| p.start).unwrap_or(0);
    let target_top = if first_element == 0 {
        0
    } else {
        cum.get(first_element - 1).copied().unwrap_or(0)
    };
    state.view_mut().scroll = max_scroll.saturating_sub(target_top);
}

fn current_top_post(snap: &crate::Snapshot) -> usize {
    crate::snapshot::compute_current_top_element(
        &snap.elements,
        &snap.line_counts,
        snap.total_lines,
        snap.scroll,
        snap.last_visible_height,
    )
    .and_then(|elem| post_for_element(snap, elem))
    .unwrap_or(0)
}

fn post_for_element(snap: &crate::Snapshot, element_index: usize) -> Option<usize> {
    snap.posts
        .iter()
        .find(|p| p.start <= element_index && element_index < p.end)
        .map(|p| p.index)
}

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
    if state.session_mut().messages.is_empty() && !state.agent_state_mut().turn_active {
        state.input_mut().input_flash = 3;
    }
    state.view_mut().scroll = state.view_mut().scroll.saturating_add(1);
}

fn scroll_down(state: &mut AppState) {
    if state.view_mut().scroll == 0 {
        state.input_mut().input_flash = 3;
    }
    state.view_mut().scroll = state.view_mut().scroll.saturating_sub(1);
}

fn page_up(state: &mut AppState) {
    if state.session_mut().messages.is_empty() && !state.agent_state_mut().turn_active {
        state.input_mut().input_flash = 3;
    }
    state.view_mut().scroll = state.view_mut().scroll.saturating_add(PAGE_SIZE);
}

fn page_down(state: &mut AppState) {
    if state.view_mut().scroll == 0 {
        state.input_mut().input_flash = 3;
    }
    state.view_mut().scroll = state.view_mut().scroll.saturating_sub(PAGE_SIZE);
}

fn go_to_top(state: &mut AppState) {
    if state.session_mut().messages.is_empty() && !state.agent_state_mut().turn_active {
        state.input_mut().input_flash = 3;
    }
    let snap = state.snapshot();
    let visible = state.view_mut().last_visible_height.max(3) as usize;
    let max_scroll = snap.total_lines.saturating_sub(visible);
    state.view_mut().scroll = max_scroll;
    if state.view_mut().vim_nav_mode {
        state.view_mut().selected_post = Some(0);
    }
}

fn go_to_bottom(state: &mut AppState) {
    let snap = state.snapshot();
    state.view_mut().scroll = 0;
    if state.view_mut().vim_nav_mode {
        let len = snap.posts.len();
        state.view_mut().selected_post = len.checked_sub(1);
    }
}
