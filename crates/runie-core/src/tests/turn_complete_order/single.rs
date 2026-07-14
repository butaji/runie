//! single tests.

use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::tests::fresh_state;
use crate::view::LazyCache;
use crate::Event;

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
            crate::view::Element::SubagentRow { .. } => "Subagent".to_string(),
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
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Hello".into(),

        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "file1".into(),

        input: None,
    });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Done".into(),

        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 2.0,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
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
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "a".into(),

        input: None,
    });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Hello ".into(),

        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "world".into(),

        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(crate::Event::Done { id: "req.0".into() });
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
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "cat".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.1,
        output: "a".into(),

        input: None,
    });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.2,
        output: "b".into(),

        input: None,
    });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Done".into(),

        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 3.0,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
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
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "file1".into(),

        input: None,
    });
    state.update(crate::Event::Done { id: "req.0".into() });
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
    state.update(crate::Event::Thinking { id: "req.0".into() });
    state.update(crate::Event::ThoughtDone { id: "req.0".into() });
    state.update(crate::Event::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "a".into(),

        input: None,
    });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Hello".into(),

        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "".into(),

        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(crate::Event::Done { id: "req.0".into() });
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
        crate::Event::Thinking { id: "req.0".into() },
        crate::Event::ThoughtDone { id: "req.0".into() },
        crate::Event::ToolStart {
            id: "req.0".into(),
            name: "ls".into(),
            input: serde_json::Value::Null,
        },
        crate::Event::ToolEnd {
            id: "".to_string(),
            duration_secs: 0.5,
            output: "a".into(),

            input: None,
        },
        crate::Event::Response {
            id: "req.0".into(),
            content: "First turn".into(),

            role: String::new(),
            timestamp: 0.0,
            provider: String::new(),
        },
        crate::Event::TurnComplete {
            id: "req.0".into(),
            duration_secs: 1.0,
        },
        crate::Event::Done { id: "req.0".into() },
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
        parts: vec![Part::Text {
            content: "Next turn".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();

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
    state.update(crate::Event::Response {
        id: "req.0".into(),
        content: "Hello".into(),

        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(crate::Event::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(crate::Event::Done { id: "req.0".into() });

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
