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
    // Set scroll beyond any possible content; rendering clamps it to the
    // actual max via Snapshot::scroll_offset.
    state.view.scroll = usize::MAX;
}

fn go_to_bottom(state: &mut AppState) {
    state.view.scroll = 0;
}
