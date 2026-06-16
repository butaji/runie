//! Safety command tests — read-only mode and trust system
use super::slash::exec;
use crate::event::{InputEvent, ModelConfigEvent, DialogEvent};
use crate::model::AppState;

pub fn fresh_state() -> AppState {
    AppState::default()
}

pub fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(InputEvent::Input(c));
    }
}

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(InputEvent::Input('/'));
    for c in cmd.chars() {
        state.update(DialogEvent::PaletteFilter(c));
    }
    state.update(DialogEvent::PaletteSelect);
}

#[test]
fn toggle_flips_read_only() {
    let mut state = fresh_state();
    assert!(!state.config.read_only, "default is read-write");
    state.update(ModelConfigEvent::ToggleReadOnly);
    assert!(state.config.read_only, "toggled to read-only");
    state.update(ModelConfigEvent::ToggleReadOnly);
    assert!(!state.config.read_only, "toggled back to read-write");
}

#[test]
fn slash_readonly_toggles() {
    let mut state = fresh_state();
    assert!(!state.config.read_only);
    palette_select(&mut state, "readonly");
    assert!(state.config.read_only, "/readonly toggles read_only");
    assert!(
        state
            .transient_message
            .as_ref()
            .unwrap()
            .contains("Read-only mode enabled"),
        "confirmation: {:?}",
        state.transient_message
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Warning)
    );
}

#[test]
fn slash_ro_alias_toggles() {
    let mut state = fresh_state();
    exec(&mut state, "/ro");
    assert!(state.config.read_only, "/ro alias toggles read_only");
}

#[test]
fn slash_trust_sets_trusted() {
    let mut state = fresh_state();
    state.config.read_only = true;
    palette_select(&mut state, "trust");
    assert!(!state.config.read_only, "/trust disables read-only");
    assert!(
        state
            .transient_message
            .as_ref()
            .unwrap()
            .contains("trusted"),
        "trust confirmation: {:?}",
        state.transient_message
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Success)
    );
}

#[test]
fn slash_untrust_sets_untrusted() {
    let mut state = fresh_state();
    palette_select(&mut state, "untrust");
    assert!(state.config.read_only, "/untrust enables read-only");
    assert!(
        state
            .transient_message
            .as_ref()
            .unwrap()
            .contains("untrusted"),
        "untrust confirmation: {:?}",
        state.transient_message
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Warning)
    );
}
