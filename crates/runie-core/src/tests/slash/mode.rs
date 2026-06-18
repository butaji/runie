//! /team and /solo slash command tests.

use super::{exec, fresh_state};
use crate::event::{DialogEvent, InputEvent};
use crate::model::AppState;
use crate::orchestrator::ExecutionMode;

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(InputEvent::Input('/'));
    for c in cmd.chars() {
        state.update(DialogEvent::PaletteFilter(c));
    }
    state.update(DialogEvent::PaletteSelect);
}

#[test]
fn team_switches_execution_mode() {
    let mut state = fresh_state();
    assert_eq!(state.config.execution_mode, ExecutionMode::Solo);
    exec(&mut state, "/team");
    assert_eq!(state.config.execution_mode, ExecutionMode::Team);
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .collect();
    assert!(
        sys.iter().any(|m| m.content.contains("Team mode")),
        "confirmation: {:?}",
        sys.last()
    );
}

#[test]
fn solo_switches_execution_mode() {
    let mut state = fresh_state();
    state.config.execution_mode = ExecutionMode::Team;
    exec(&mut state, "/solo");
    assert_eq!(state.config.execution_mode, ExecutionMode::Solo);
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .collect();
    assert!(
        sys.iter().any(|m| m.content.contains("Solo mode")),
        "confirmation: {:?}",
        sys.last()
    );
}

#[test]
fn team_hint_reflects_mode() {
    let mut state = fresh_state();
    state.config.execution_mode = ExecutionMode::Team;
    let hint = state.hint_text();
    assert!(
        hint.contains("ctrl+0"),
        "Team mode hint should show orchestrator hotkey: {}",
        hint
    );
}

#[test]
fn solo_hint_does_not_show_team_hotkeys() {
    let mut state = fresh_state();
    let hint = state.hint_text();
    assert!(
        !hint.contains("ctrl+0"),
        "Solo mode hint should not show orchestrator hotkey: {}",
        hint
    );
}

#[test]
fn team_palette_select_switches_mode() {
    let mut state = AppState::default();
    palette_select(&mut state, "team");
    assert_eq!(state.config.execution_mode, ExecutionMode::Team);
}
