//! Tests for the /spawn command.
//!
//! The /spawn command lives in runie-core but the actual subagent execution
//! is in runie-agent. The command emits a SpawnAgent event; the binary
//! layer catches it and runs the subagent.

use crate::event::Event;

use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use crate::model::AppState;

/// Set input buffer directly and submit — bypasses the command palette.
fn exec(state: &mut AppState, text: &str) {
    state.input.input = text.into();
    state.input.cursor_pos = text.len();
    state.update(Event::submit());
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
fn spawn_without_args_opens_form_no_chat_message() {
    let mut state = AppState::default();
    exec(&mut state, "/spawn");

    // Should NOT add a system message to the chat feed.
    let sys_count = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .count();
    assert_eq!(
        sys_count, 0,
        "/spawn without args should not add chat messages"
    );

    // Should open a dialog (form).
    assert!(
        state.open_dialog.is_some(),
        "/spawn without args should open a form dialog"
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
        crate::commands::CommandResult::Event(Event::Control(ControlEvent::SpawnAgent { prompt })) => {
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
