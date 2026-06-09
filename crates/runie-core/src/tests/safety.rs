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
    assert!(!state.read_only, "default is read-write");
    state.update(Event::ToggleReadOnly);
    assert!(state.read_only, "toggled to read-only");
    state.update(Event::ToggleReadOnly);
    assert!(!state.read_only, "toggled back to read-write");
}

#[test]
fn slash_readonly_toggles() {
    let mut state = fresh_state();
    assert!(!state.read_only);
    type_str(&mut state, "/readonly");
    state.update(Event::Submit);
    assert!(state.read_only, "/readonly toggles read_only");
    let sys_msgs: Vec<_> = state.messages.iter().filter(|m| m.role == Role::System).collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(last.content.contains("Read-only mode enabled"), "confirmation: {}", last.content);
}

#[test]
fn slash_ro_alias_toggles() {
    let mut state = fresh_state();
    type_str(&mut state, "/ro");
    state.update(Event::Submit);
    assert!(state.read_only, "/ro alias toggles read_only");
}

#[test]
fn slash_trust_sets_trusted() {
    let mut state = fresh_state();
    state.read_only = true;
    type_str(&mut state, "/trust");
    state.update(Event::Submit);
    assert!(!state.read_only, "/trust disables read-only");
    let sys_msgs: Vec<_> = state.messages.iter().filter(|m| m.role == Role::System).collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(last.content.contains("trusted"), "trust confirmation: {}", last.content);
}

#[test]
fn slash_untrust_sets_untrusted() {
    let mut state = fresh_state();
    type_str(&mut state, "/untrust");
    state.update(Event::Submit);
    assert!(state.read_only, "/untrust enables read-only");
    let sys_msgs: Vec<_> = state.messages.iter().filter(|m| m.role == Role::System).collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(last.content.contains("untrusted"), "untrust confirmation: {}", last.content);
}
