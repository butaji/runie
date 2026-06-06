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

    assert_eq!(state.messages.len(), 1);
    assert!(state.messages[0].content.contains("State cleared"), "reset confirmation: {}", state.messages[0].content);
    assert_eq!(state.input, "");
}

#[test]
fn at_ref_tracks_last_query() {
    let mut state = fresh_state();
    state.update(Event::Input('@'));
    assert_eq!(state.last_at_query, Some("".to_string()), "Empty query after @");

    state.update(Event::Input('C'));
    assert_eq!(state.last_at_query, Some("C".to_string()), "Query should be 'C'");
}

#[test]
fn typing_without_at_clears_query_tracker() {
    let mut state = fresh_state();
    for c in "hello".chars() {
        state.update(Event::Input(c));
    }
    assert!(state.at_suggestions.is_none(), "Typing without @ should not trigger suggestions");
    assert!(state.last_at_query.is_none());
}
