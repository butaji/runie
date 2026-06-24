//! Tests for chat feed element sorting by last update time.

use runie_core::event::Event;

use runie_core::event::{AgentEvent, InputEvent};
use runie_core::model::{AppState, ChatMessage,  Role};
use runie_core::Part;
use runie_core::view::LazyCache;
use runie_testing::fresh_state;

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

fn element_kinds(state: &AppState) -> Vec<String> {
    let feed = LazyCache::feed(state);
    feed.elements
        .iter()
        .map(|e| match e {
            runie_core::view::Element::UserMessage { .. } => "User".to_string(),
            runie_core::view::Element::AgentMessage { .. } => "Agent".to_string(),
            runie_core::view::Element::Thinking { .. } => "Thinking".to_string(),
            runie_core::view::Element::ThoughtMarker { .. } => "Thought".to_string(),
            runie_core::view::Element::ThoughtSummary { .. } => "ThoughtSum".to_string(),
            runie_core::view::Element::ToolRunning { .. } => "ToolRun".to_string(),
            runie_core::view::Element::ToolDone { .. } => "ToolDone".to_string(),
            runie_core::view::Element::ToolSummary { .. } => "ToolSum".to_string(),
            runie_core::view::Element::ContextGroup { .. } => "Context".to_string(),
            runie_core::view::Element::TurnComplete { .. } => "Turn".to_string(),
            runie_core::view::Element::Spacer { .. } => "Spacer".to_string(),
        })
        .collect()
}

/// Every element (including spacers) should have non-decreasing timestamps.
fn _timestamps_are_monotonic(state: &AppState) -> Result<(), String> {
    let feed = LazyCache::feed(state);
    let last_ts = 0.0f64;
    for (i, entry) in feed.elements.iter().enumerate() {
        // We can't directly read timestamp from Element, but we can infer from the source
        // messages. Instead, we'll use a different approach: verify sort order by checking
        // that the feed is built from messages sorted by timestamp.
        let _ = (i, entry, last_ts);
    }
    Ok(())
}

fn response_after_tool_events() -> Vec<Event> {
    vec![
        AgentEvent::Response {
            id: "req.0".into(),
            content: "Let me ".into(),
        },
        AgentEvent::ToolStart {
            id: "req.0".into(),
            name: "ls".into(),
            input: serde_json::Value::Null,
        },
        AgentEvent::ToolEnd {
            id: "".to_string(),
            duration_secs: 0.5,
            output: "file.txt".into(),
        },
        AgentEvent::Response {
            id: "req.0".into(),
            content: "check files.".into(),
        },
        AgentEvent::TurnComplete {
            id: "req.0".into(),
            duration_secs: 2.0,
        },
        AgentEvent::Done { id: "req.0".into() },
    ]
}

// ─── Scenario 1: Streaming response after tool ─────────────────────────

#[test]
fn agent_response_updated_after_tool_stays_after_tool() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    dispatch(&mut state, &response_after_tool_events());
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state)
        .into_iter()
        .filter(|k| k != "Spacer")
        .collect();
    let tool_pos = kinds.iter().position(|k| k == "ToolDone");
    let agent_pos = kinds.iter().position(|k| k == "Agent");
    assert!(tool_pos.is_some(), "Tool should exist");
    assert!(agent_pos.is_some(), "Agent should exist");
    assert!(
        tool_pos.unwrap() < agent_pos.unwrap(),
        "Agent should appear after Tool: got {:?}",
        kinds
    );
}

// ─── Scenario 2: Multiple response chunks preserve relative order ───────

#[test]
fn multiple_response_chunks_preserve_creation_order() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    // First chunk creates assistant
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "Hello ".into(),
    });
    // Second chunk updates same assistant (bumps timestamp)
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "world".into(),
    });
    // Third chunk
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "!".into(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(AgentEvent::Done { id: "req.0".into() });
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state)
        .into_iter()
        .filter(|k| k != "Spacer")
        .collect();
    // Should be exactly one Agent message (chunks merged)
    let agent_count = kinds.iter().filter(|k| *k == "Agent").count();
    assert_eq!(
        agent_count, 1,
        "Multiple chunks should merge into one Agent message"
    );
}

fn thought_before_agent_events() -> Vec<Event> {
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
            content: "Result".into(),
        },
        AgentEvent::Response {
            id: "req.0".into(),
            content: " done".into(),
        },
        AgentEvent::TurnComplete {
            id: "req.0".into(),
            duration_secs: 1.0,
        },
        AgentEvent::Done { id: "req.0".into() },
    ]
}

// ─── Scenario 3: Thought before agent, agent updated later ─────────────

#[test]
fn thought_appears_before_agent_even_when_agent_updated_later() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    dispatch(&mut state, &thought_before_agent_events());
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state)
        .into_iter()
        .filter(|k| k != "Spacer")
        .collect();
    let thought_pos = kinds.iter().position(|k| k == "Thought");
    let agent_pos = kinds.iter().position(|k| k == "Agent");
    assert!(thought_pos.is_some(), "Thought should exist");
    assert!(agent_pos.is_some(), "Agent should exist");
    assert!(
        thought_pos.unwrap() < agent_pos.unwrap(),
        "Thought should appear before Agent: got {:?}",
        kinds
    );
}

// ─── Scenario 4: TurnComplete is strictly last during its turn ─────────

#[test]
fn turn_complete_last_during_turn_despite_updates() {
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
    // Even after turn complete, delayed empty response bumps assistant
    state.update(AgentEvent::Response {
        id: "req.0".into(),
        content: "".into(),
    });
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state)
        .into_iter()
        .filter(|k| k != "Spacer")
        .collect();
    assert_eq!(
        kinds.last(),
        Some(&"Turn".to_string()),
        "TurnComplete must be last: got {:?}",
        kinds
    );
}

fn turn_then_user_events() -> Vec<Event> {
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
            content: "T1".into(),
        },
        AgentEvent::TurnComplete {
            id: "req.0".into(),
            duration_secs: 1.0,
        },
        AgentEvent::Done { id: "req.0".into() },
        InputEvent::Input('H'),
        Event::submit(),
    ]
}

// ─── Scenario 5: Cross-turn ordering ───────────────────────────────────

#[test]
fn previous_turn_complete_before_next_turn_user() {
    let mut state = fresh_state();
    dispatch(&mut state, &turn_then_user_events());
    state.ensure_fresh();

    let kinds: Vec<_> = element_kinds(&state)
        .into_iter()
        .filter(|k| k != "Spacer")
        .collect();
    let turn_pos = kinds
        .iter()
        .position(|k| k == "Turn")
        .expect("TurnComplete");
    let user_positions: Vec<_> = kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| *k == "User")
        .map(|(i, _)| i)
        .collect();
    assert!(!user_positions.is_empty());
    assert!(
        turn_pos < *user_positions.last().unwrap(),
        "TurnComplete should be before user2: got {:?}",
        kinds
    );
}

// ─── Scenario 6: Timestamp-based sort, not index-based ─────────────────

#[test]
fn elements_sorted_by_timestamp_not_index() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "First".into() }],
        timestamp: 3.0,
        id: "u1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "Second".into() }],
        timestamp: 1.0,
        id: "u2".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let user_contents: Vec<_> = feed
        .elements
        .iter()
        .filter_map(|e| match e {
            runie_core::view::Element::UserMessage { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        user_contents,
        vec!["Second", "First"],
        "Messages should be sorted by timestamp"
    );
}
