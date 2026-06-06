use crate::model::AppState;
use crate::event::Event;
use crate::update::update;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_input_adds_character() {
    let state = fresh_state();
    let state = update(state, Event::Input('H'));
    let state = update(state, Event::Input('i'));
    assert_eq!(state.input, "Hi");
}

#[test]
fn test_backspace_removes_character() {
    let state = fresh_state();
    let state = update(state, Event::Input('H'));
    let state = update(state, Event::Input('i'));
    let state = update(state, Event::Backspace);
    assert_eq!(state.input, "H");
}

#[test]
fn test_backspace_empty_input() {
    let state = fresh_state();
    let state = update(state, Event::Backspace);
    assert_eq!(state.input, "");
}

#[test]
fn test_submit_empty_input() {
    let state = fresh_state();
    let state = update(state, Event::Submit);
    assert_eq!(state.input, "");
}

#[test]
fn test_submit_reset_command() {
    let state = update(update(fresh_state(), Event::Input('/')), Event::Input('r'));
    let state = update(state, Event::Input('e'));
    let state = update(state, Event::Input('s'));
    let state = update(state, Event::Input('e'));
    let state = update(state, Event::Input('t'));
    let state = update(state, Event::Submit);
    assert_eq!(state.messages.len(), 0);
    assert_eq!(state.input, "");
}
