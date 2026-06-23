//! Layer 2 tests for Shift+Enter multiline input.

use crate::event::{Event, InputEvent};
use crate::model::{AppState, Role};

#[test]
fn shift_enter_inserts_newline_without_submitting() {
    let mut state = AppState::default();
    state.update(InputEvent::Input('h'));
    state.update(InputEvent::Input('i'));
    state.update(Event::newline());
    assert_eq!(state.input.input, "hi\n");
    assert_eq!(state.session.messages.len(), 0);
}

#[test]
fn shift_enter_then_enter_submits_multiline_message() {
    let mut state = AppState::default();
    for c in "line one".chars() {
        state.update(InputEvent::Input(c));
    }
    state.update(Event::newline());
    for c in "line two".chars() {
        state.update(InputEvent::Input(c));
    }
    state.update(Event::submit());

    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].role, Role::User);
    assert_eq!(state.session.messages[0].content(), "line one\nline two");
    assert!(state.input.input.is_empty());
}

#[test]
fn multiline_input_is_trimmed_on_submit() {
    let mut state = AppState::default();
    state.update(Event::newline());
    for c in "content".chars() {
        state.update(InputEvent::Input(c));
    }
    state.update(Event::newline());
    state.update(Event::submit());

    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].content(), "content");
}
