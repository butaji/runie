use crate::model::AppState;
use crate::event::Event;
use crate::update::update;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_reset_clears_state() {
    let mut state = fresh_state();
    state.input = "test".to_string();
    state.streaming = true;
    let state = update(state, Event::Reset);
    assert_eq!(state.input, "");
    assert!(!state.streaming);
    assert_eq!(state.messages.len(), 0);
}

#[test]
fn test_scroll_up() {
    let state = fresh_state();
    let state = update(state, Event::ScrollUp);
    assert_eq!(state.scroll, 1);
}

#[test]
fn test_scroll_down() {
    let mut state = fresh_state();
    state.scroll = 5;
    let state = update(state, Event::ScrollDown);
    assert_eq!(state.scroll, 4);
}

#[test]
fn test_scroll_down_saturates() {
    let mut state = fresh_state();
    state.scroll = 0;
    let state = update(state, Event::ScrollDown);
    assert_eq!(state.scroll, 0);
}

#[test]
fn test_messages_have_correlation_id() {
    let state = fresh_state();
    let state = update(state, Event::Input('H'));
    let state = update(state, Event::Submit);
    assert_eq!(state.messages.len(), 1);
    assert!(state.messages[0].id.starts_with("req."));
}

#[test]
fn test_multiple_submits_increment_id() {
    let state = fresh_state();
    let state = update(state, Event::Input('A'));
    let state = update(state, Event::Submit);
    let first_id = state.messages[0].id.clone();
    let state = update(state, Event::Input('B'));
    let state = update(state, Event::Submit);
    let second_id = state.messages[1].id.clone();
    assert_ne!(first_id, second_id);
}
