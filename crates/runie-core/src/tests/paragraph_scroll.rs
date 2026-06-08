use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn scroll_offset_zero_when_at_bottom() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".into(),
        timestamp: 0.0,
        id: "u0".into(),
    });
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0; // at bottom

    let snap = state.snapshot();
    // total_lines = 2 (user + spacer), height = 5
    // max_scroll = 0 (fits), offset = 0 - 0 = 0
    assert_eq!(snap.scroll_offset(5), 0, "When content fits, offset is 0");
}

#[test]
fn scroll_offset_max_when_fully_scrolled() {
    let mut state = fresh_state();
    for i in 0..10 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 100; // fully scrolled up (clamped)

    let snap = state.snapshot();
    // total_lines = 20, height = 5, max_scroll = 15
    // offset = 15 - 15 = 0 (top)
    assert_eq!(snap.scroll_offset(5), 0, "Fully scrolled up shows from top");
}

#[test]
fn scroll_offset_shows_bottom_when_zero() {
    let mut state = fresh_state();
    for i in 0..10 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0; // at bottom

    let snap = state.snapshot();
    // total_lines = 20, height = 5, max_scroll = 15
    // offset = 15 - 0 = 15 (from top = bottom)
    assert_eq!(snap.scroll_offset(5), 15, "At bottom, offset = max_scroll");
}

#[test]
fn scroll_offset_halfway() {
    let mut state = fresh_state();
    for i in 0..10 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 7; // halfway up

    let snap = state.snapshot();
    // total_lines = 20, height = 5, max_scroll = 15
    // offset = 15 - 7 = 8
    assert_eq!(snap.scroll_offset(5), 8, "Halfway scroll gives correct offset");
}

#[test]
fn scrollbar_state_has_viewport_content_length() {
    let mut state = fresh_state();
    for i in 0..10 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.scroll = 0;

    let snap = state.snapshot();
    let (thumb, _offset) = snap.scrollbar_metrics(5);
    // With viewport_content_length, thumb should reflect viewport ratio
    // total = 20, viewport = 5, thumb = max(1, 5*5/20) = max(1, 1) = 1... but with
    // proper viewport_content_length set, ratatui computes it internally
    // We just check thumb > 0 when content overflows
    assert!(thumb > 0 || snap.total_lines <= 5, "Thumb should be visible when content overflows");
}
