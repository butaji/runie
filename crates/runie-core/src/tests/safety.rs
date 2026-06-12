//! Safety command tests — read-only mode and trust system
use crate::model::{AppState, Role};
use crate::event::Event;

pub fn fresh_state() -> AppState {
    AppState::default()
}

pub fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(Event::Input(c));
    }
}

#[test]
fn toggle_flips_read_only() {
    let mut state = fresh_state();
    assert!(!state.config.read_only, "default is read-write");
    state.update(Event::ToggleReadOnly);
    assert!(state.config.read_only, "toggled to read-only");
    state.update(Event::ToggleReadOnly);
    assert!(!state.config.read_only, "toggled back to read-write");
}

#[test]
fn slash_readonly_toggles() {
    let mut state = fresh_state();
    assert!(!state.config.read_only);
    type_str(&mut state, "/readonly");
    state.update(Event::Submit);
    assert!(state.config.read_only, "/readonly toggles read_only");
    assert!(state.transient_message.as_ref().unwrap().contains("Read-only mode enabled"), "confirmation: {:?}", state.transient_message);
    assert_eq!(state.transient_level, Some(crate::event::TransientLevel::Warning));
}

#[test]
fn slash_ro_alias_toggles() {
    let mut state = fresh_state();
    type_str(&mut state, "/ro");
    state.update(Event::Submit);
    assert!(state.config.read_only, "/ro alias toggles read_only");
}

#[test]
fn slash_trust_sets_trusted() {
    let mut state = fresh_state();
    state.config.read_only = true;
    type_str(&mut state, "/trust");
    state.update(Event::Submit);
    assert!(!state.config.read_only, "/trust disables read-only");
    assert!(state.transient_message.as_ref().unwrap().contains("trusted"), "trust confirmation: {:?}", state.transient_message);
    assert_eq!(state.transient_level, Some(crate::event::TransientLevel::Success));
}

#[test]
fn slash_untrust_sets_untrusted() {
    let mut state = fresh_state();
    type_str(&mut state, "/untrust");
    state.update(Event::Submit);
    assert!(state.config.read_only, "/untrust enables read-only");
    assert!(state.transient_message.as_ref().unwrap().contains("untrusted"), "untrust confirmation: {:?}", state.transient_message);
    assert_eq!(state.transient_level, Some(crate::event::TransientLevel::Warning));
}
