//! single tests.

use crate::event::Event;

use crate::event::AgentEvent;
use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
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

#[test]
fn turn_complete_is_last_after_normal_flow() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Hello".into(),
    });
    state.update(AgentEvent::ThoughtDone { id: "req.0".into() });
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "file1".into(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done".into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 2.0,
    });
    state.update(AgentEvent::Done { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must be the last element in the turn: got {:?}",
        kinds
    );
}

#[test]
fn turn_complete_is_last_when_response_after_turn_complete() {
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
        content: "Hello ".into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "world".into(),
    });
    state.update(AgentEvent::Done { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must remain last even when response chunks arrive after it: got {:?}",
        kinds
    );
}

#[test]
fn turn_complete_is_last_with_multiple_tools() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    state.update(AgentEvent::ThoughtDone { id: "req.0".into() });
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "cat".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.1,
        output: "a".into(),
    });
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.2,
        output: "b".into(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done".into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 3.0,
    });
    state.update(AgentEvent::Done { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must be last after multiple tools: got {:?}",
        kinds
    );
}

#[test]
fn turn_complete_is_last_when_tool_end_after_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking { id: "req.0".into() });
    state.update(AgentEvent::ThoughtDone { id: "req.0".into() });
    state.update(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "file1".into(),
    });
    state.update(AgentEvent::Done { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must remain last even when tool end arrives after it: got {:?}",
        kinds
    );
}

#[test]
fn turn_complete_survives_empty_content_timestamp_bump() {
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
        id: "req.0".into(),
        content: "".into(),
    });
    state.update(AgentEvent::Done { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must remain last after empty response timestamp bump: got {:?}",
        kinds
    );
}

fn first_turn_with_tool_events() -> Vec<Event> {
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
            content: "First turn".into(),
        },
        AgentEvent::TurnComplete {
            id: "req.0".into(),
            duration_secs: 1.0,
        },
        AgentEvent::Done { id: "req.0".into() },
    ]
}

#[test]
fn turn_complete_before_next_turn_user_message() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    dispatch(&mut state, &first_turn_with_tool_events());
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: crate::model::now(),
        id: "u1".into(),
        parts: vec![Part::Text { content: "Next turn".into() }],
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    let turn_pos = kinds
        .iter()
        .position(|k| k == "Turn")
        .expect("TurnComplete should exist");
    let user_pos = kinds
        .iter()
        .position(|k| k == "User")
        .expect("User should exist");
    assert!(
        turn_pos < user_pos,
        "TurnComplete should be before user2: got {:?}",
        kinds
    );
}

#[test]
fn turn_complete_timestamp_is_max_after_done() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Hello".into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(AgentEvent::Done { id: "req.0".into() });

    let turn_ts = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::TurnComplete)
        .map(|m| m.timestamp)
        .unwrap();
    let max_other_ts = state
        .session
        .messages
        .iter()
        .filter(|m| m.role != Role::TurnComplete)
        .map(|m| m.timestamp)
        .fold(0.0, f64::max);

    assert!(turn_ts > max_other_ts,
        "TurnComplete timestamp ({}) must be strictly greater than all other messages ({}) after finish_turn",
        turn_ts, max_other_ts);
}
