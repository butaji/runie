use crate::model::AppState;
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_input_adds_character() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    assert_eq!(state.input, "Hi");
}

#[test]
fn test_backspace_removes_character() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Backspace);
    assert_eq!(state.input, "H");
}

#[test]
fn test_backspace_empty_input() {
    let mut state = fresh_state();
    state.update(Event::Backspace);
    assert_eq!(state.input, "");
}

#[test]
fn test_submit_empty_input() {
    let mut state = fresh_state();
    state.update(Event::Submit);
    assert_eq!(state.input, "");
}

#[test]
fn test_submit_reset_command() {
    let mut state = fresh_state();
    state.update(Event::Input('/'));
    state.update(Event::Input('r'));
    state.update(Event::Input('e'));
    state.update(Event::Input('s'));
    state.update(Event::Input('e'));
    state.update(Event::Input('t'));
    state.update(Event::Submit);
    assert_eq!(state.messages.len(), 0);
    assert_eq!(state.input, "");
}

#[test]
fn test_input_events_mark_dirty_for_render() {
    // REGRESSION: push_input/pop_input/scroll did not call mark_dirty()
    // In actor architecture, maybe_send_snapshot gates on is_dirty().
    // Without dirty, typing never triggered render — TUI stayed blank.
    let mut state = fresh_state();
    state.ensure_fresh(); // consume initial dirty
    assert!(!state.is_dirty());

    state.update(Event::Input('a'));
    assert!(state.is_dirty(), "Input must mark dirty");

    state.ensure_fresh();
    assert!(!state.is_dirty());

    state.update(Event::Backspace);
    assert!(state.is_dirty(), "Backspace must mark dirty");

    state.ensure_fresh();
    assert!(!state.is_dirty());

    state.update(Event::ScrollUp);
    assert!(state.is_dirty(), "ScrollUp must mark dirty");

    state.ensure_fresh();
    assert!(!state.is_dirty());

    state.update(Event::ScrollDown);
    assert!(state.is_dirty(), "ScrollDown must mark dirty");
}
