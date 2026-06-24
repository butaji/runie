//! multi tests.

use crate::event::Event;

use crate::event::AgentEvent;
use crate::model::{AppState, Role};
use crate::view::LazyCache;
use crate::tests::fresh_state;

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

fn element_kinds_no_spacer(state: &AppState) -> Vec<String> {
    let feed = LazyCache::feed(state);
    feed.elements
        .iter()
        .map(|e| match e {
            crate::view::Element::UserMessage { .. } => "User".to_string(),
            crate::view::Element::AgentMessage { .. } => "Agent".to_string(),
            crate::view::Element::Thinking { .. } => "Thinking".to_string(),
            crate::view::Element::ThoughtMarker { .. } => "Thought".to_string(),
            crate::view::Element::ThoughtSummary { .. } => "ThoughtSum".to_string(),
            crate::view::Element::ToolRunning { .. } => "ToolRun".to_string(),
            crate::view::Element::ToolDone { .. } => "ToolDone".to_string(),
            crate::view::Element::ToolSummary { .. } => "ToolSum".to_string(),
            crate::view::Element::ContextGroup { .. } => "Context".to_string(),
            crate::view::Element::TurnComplete { .. } => "Turn".to_string(),
            crate::view::Element::Spacer { .. } => "Spacer".to_string(),
        })
        .filter(|k| k != "Spacer")
        .collect()
}

fn duplicate_turn_complete_events() -> Vec<Event> {
    vec![
        AgentEvent::Thinking { id: "req.0".into() },
        AgentEvent::ThoughtDone { id: "req.0".into() },
        AgentEvent::ToolStart {
            id: "req.0".into(),
            name: "ls".into(),
            input: serde_json::Value::Null,
        },
        AgentEvent::ToolEnd {
            id: "".to_string(),
            duration_secs: 0.5,
            output: "a".into(),
        },
        AgentEvent::Response {
            id: "req.0".into(),
            content: "Hello".into(),
        },
        AgentEvent::TurnComplete {
            id: "req.0".into(),
            duration_secs: 1.0,
        },
        AgentEvent::TurnComplete {
            id: "req.0".into(),
            duration_secs: 1.0,
        },
        AgentEvent::Done { id: "req.0".into() },
    ]
}

#[test]
fn turn_complete_deduplicated_on_duplicate_events() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    dispatch(&mut state, &duplicate_turn_complete_events());
    state.ensure_fresh();

    let turn_count = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::TurnComplete)
        .count();
    assert_eq!(
        turn_count, 1,
        "Should have exactly one TurnComplete, got {}",
        turn_count
    );

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()));
}

#[test]
fn turn_complete_is_last_when_new_assistant_after_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    state.update(AgentEvent::ThoughtDone { id: "req.0".into() });
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "a".into(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Hello".into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(AgentEvent::Response {
        id: "req.1".into(),
        content: "Delayed".into(),
    });
    state.update(AgentEvent::Done { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must remain last even when new assistant message arrives after it: got {:?}",
        kinds
    );
}

#[test]
fn turn_complete_is_last_when_error_after_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    state.update(AgentEvent::ThoughtDone { id: "req.0".into() });
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "a".into(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Hello".into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(AgentEvent::Error {
        id: "req.0".into(),
        message: "Oops".into(),
    });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must remain last even when error arrives after it: got {:?}",
        kinds
    );
}

#[test]
fn turn_complete_is_last_when_response_after_done() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    state.update(AgentEvent::ThoughtDone { id: "req.0".into() });
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "a".into(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Hello".into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(AgentEvent::Done { id: "req.0".into() });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "world".into(),
    });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must remain last even when response arrives after AgentDone: got {:?}",
        kinds
    );
}

#[test]
fn turn_complete_is_last_when_thinking_after_done() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    state.update(AgentEvent::ThoughtDone { id: "req.0".into() });
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "a".into(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Hello".into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(AgentEvent::Done { id: "req.0".into() });
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must remain last even when thinking arrives after AgentDone: got {:?}",
        kinds
    );
}

fn run_turn(state: &mut AppState, id: &str, tool_name: &str, agent_content: &str) {
    state.update(AgentEvent::Thinking { id: id.into() });
    state.update(AgentEvent::ThoughtDone { id: id.into() });
    state.update(AgentEvent::ToolStart {
        id: id.into(),
        name: tool_name.into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.3,
        output: "x".into(),
    });
    state.update(AgentEvent::Response {
        id: id.into(),
        content: agent_content.into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: id.into(),
        duration_secs: 1.0,
    });
    state.update(AgentEvent::Done { id: id.into() });
}

/// Bug: move_turn_complete_to_end() moved ANY TurnComplete, causing
/// turn 1's TurnComplete to leapfrog turn 2's content.
#[test]
fn turn_complete_order_preserved_across_multiple_turns() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    run_turn(&mut state, "req.0", "ls", "First turn");
    run_turn(&mut state, "req.1", "cat", "Second turn");
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    let turn_positions: Vec<usize> = kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| *k == "Turn")
        .map(|(i, _)| i)
        .collect();

    assert_eq!(
        turn_positions.len(),
        2,
        "Expected 2 TurnComplete, got {:?}",
        kinds
    );
    let (turn1_pos, turn2_pos) = (turn_positions[0], turn_positions[1]);
    assert!(
        turn1_pos < turn2_pos,
        "Turn 1 (pos {}) must be before turn 2 (pos {}): got {:?}",
        turn1_pos,
        turn2_pos,
        kinds
    );

    let second_agent_pos = kinds.iter().rposition(|k| k == "Agent").unwrap();
    assert!(
        turn1_pos < second_agent_pos,
        "Turn 1 must be before turn 2's agent: got {:?}",
        kinds
    );
}
