use crate::model::AppState;
use crate::event::Event;

#[test]
fn at_ref_shows_suggestions() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    for c in "Cargo".chars() {
        state.update(Event::Input(c));
    }
    let suggestions = state.completion.at_suggestions.clone().unwrap_or_default();
    assert!(!suggestions.is_empty(), "Should find files matching @Cargo");
    assert!(suggestions.iter().any(|s| s.contains("Cargo")));
}

#[test]
fn at_ref_empty_shows_files() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    let suggestions = state.completion.at_suggestions.clone().unwrap_or_default();
    assert!(!suggestions.is_empty(), "Should list files after @");
}

#[test]
fn tab_cycles_suggestions() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    state.update(Event::Input('C'));
    state.update(Event::Input('\t'));
    let first = state.completion.at_suggestions.clone().unwrap_or_default();
    assert!(!first.is_empty());
    let idx1 = state.completion.at_selected.unwrap_or(0);
    state.update(Event::Input('\t'));
    let idx2 = state.completion.at_selected.unwrap_or(0);
    assert_ne!(idx1, idx2, "Tab should cycle to next suggestion");
}

#[test]
fn enter_inserts_selected_suggestion() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    for c in "Cargo.toml".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Input('\t'));
    let suggestions = state.completion.at_suggestions.clone().unwrap_or_default();
    assert!(!suggestions.is_empty());
    state.update(Event::Submit);
    assert!(!state.input.input.contains('@'), "@ should be replaced by selected file");
    assert!(state.input.input.contains("Cargo.toml"), "Expected Cargo.toml in {:?}", state.input.input);
}

#[test]
fn escape_clears_at_suggestions() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    state.update(Event::Input('C'));
    state.update(Event::Input('\t'));
    assert!(state.completion.at_suggestions.is_some());
    state.update(Event::Abort);
    assert!(state.completion.at_suggestions.is_none());
}

#[test]
fn no_at_ref_no_suggestions() {
    let state = AppState::default();
    assert!(state.completion.at_suggestions.is_none());
}
