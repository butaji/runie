//! Tests for the /spawn command.
//!
//! The /spawn command lives in runie-core but the actual subagent execution
//! is in runie-agent. The command emits a SpawnAgent event; the binary
//! layer catches it and runs the subagent.

use crate::event::Event;
use crate::model::AppState;

/// Set input buffer directly and submit — bypasses the command palette.
fn exec(state: &mut AppState, text: &str) {
    state.input.input = text.into();
    state.input.cursor_pos = text.len();
    state.update(Event::Submit);
}

#[test]
fn spawn_command_is_registered() {
    let state = AppState::default();
    assert!(
        state.registry.get("spawn").is_some(),
        "/spawn must be registered in the command registry"
    );
}

#[test]
fn spawn_without_args_shows_usage() {
    let mut state = AppState::default();
    exec(&mut state, "/spawn");

    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .collect();
    assert!(!sys.is_empty(), "expected a system message");
    let last = sys.last().unwrap().content.to_lowercase();
    assert!(
        last.contains("usage") || last.contains("spawn"),
        "expected usage hint, got: {:?}",
        last
    );
}

#[test]
fn spawn_emits_spawn_agent_event() {
    let mut state = AppState::default();
    let cmd = state.registry.get("spawn").expect("registered");
    let cmd_name = cmd.name.clone();
    let result = cmd
        .flow
        .clone()
        .exec(&mut state, &cmd_name, "find all TODOs");
    match result {
        crate::commands::CommandResult::Event(Event::SpawnAgent { prompt }) => {
            assert_eq!(prompt, "find all TODOs");
        }
        other => panic!("expected SpawnAgent event, got: {:?}", other),
    }
}

#[test]
fn spawn_event_round_trips_through_state() {
    // End-to-end: type /spawn with prompt, Submit, then inspect the
    // most recent system message. The SpawnAgent event is "handled" by
    // the no-op match in update() (binary layer is responsible for
    // actually running the subagent). The system message is whatever
    // the command produced — for /spawn that is the SpawnAgent event
    // being dispatched but not producing user-visible output. The
    // important assertion is that no panic occurs.
    let mut state = AppState::default();
    exec(&mut state, "/spawn do something");
    // No crash; state is still consistent.
    assert!(state.open_dialog.is_none());
}
