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
