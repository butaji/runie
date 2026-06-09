use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;
use crate::ui::LazyCache;

fn fresh_state() -> AppState {
    AppState::default()
}

fn element_kinds_no_spacer(state: &AppState) -> Vec<String> {
    let feed = LazyCache::feed(state);
    feed.elements.iter().map(|e| match e {
        crate::ui::Element::UserMessage { .. } => "User".to_string(),
        crate::ui::Element::AgentMessage { .. } => "Agent".to_string(),
        crate::ui::Element::Thinking { .. } => "Thinking".to_string(),
        crate::ui::Element::ThoughtMarker { .. } => "Thought".to_string(),
        crate::ui::Element::ThoughtSummary { .. } => "ThoughtSum".to_string(),
        crate::ui::Element::ToolRunning { .. } => "ToolRun".to_string(),
        crate::ui::Element::ToolDone { .. } => "ToolDone".to_string(),
        crate::ui::Element::ToolSummary { .. } => "ToolSum".to_string(),
        crate::ui::Element::TurnComplete { .. } => "Turn".to_string(),
        crate::ui::Element::Spacer { .. } => "Spacer".to_string(),
    }).filter(|k| k != "Spacer").collect()
}

#[test]
fn turn_complete_is_last_after_normal_flow() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 2.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must be the last element in the turn: got {:?}", kinds);
}

#[test]
fn turn_complete_is_last_when_response_after_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello ".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    // Simulates a delayed/buffered response chunk arriving after turn complete
    state.update(Event::AgentResponse { id: "req.0".into(), content: "world".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last even when response chunks arrive after it: got {:?}", kinds);
}

#[test]
fn turn_complete_is_last_with_multiple_tools() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "cat".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.1, output: "a".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.2, output: "b".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 3.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must be last after multiple tools: got {:?}", kinds);
}

#[test]
fn turn_complete_is_last_when_tool_end_after_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    // Delayed tool end arrives after turn complete
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last even when tool end arrives after it: got {:?}", kinds);
}

#[test]
fn turn_complete_survives_empty_content_timestamp_bump() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    // Empty append response bumps assistant timestamp
    state.update(Event::AgentResponse { id: "req.0".into(), content: "".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last after empty response timestamp bump: got {:?}", kinds);
}

#[test]
fn turn_complete_before_next_turn_user_message() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "First turn".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    // User sends next message
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "Next turn".into(),
        timestamp: crate::model::now(),
        id: "u1".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    let turn_pos = kinds.iter().position(|k| k == "Turn").expect("TurnComplete should exist");
    let user_pos = kinds.iter().position(|k| k == "User").expect("User should exist");
    assert!(turn_pos < user_pos,
        "TurnComplete of previous turn must appear before next turn's user message: got {:?}", kinds);
}

#[test]
fn turn_complete_timestamp_is_max_after_done() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });

    let turn_ts = state.messages.iter()
        .find(|m| m.role == Role::TurnComplete)
        .map(|m| m.timestamp)
        .unwrap();
    let max_other_ts = state.messages.iter()
        .filter(|m| m.role != Role::TurnComplete)
        .map(|m| m.timestamp)
        .fold(0.0, f64::max);

    assert!(turn_ts > max_other_ts,
        "TurnComplete timestamp ({}) must be strictly greater than all other messages ({}) after finish_turn",
        turn_ts, max_other_ts);
}

#[test]
fn turn_complete_deduplicated_on_duplicate_events() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 }); // duplicate
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let turn_count = state.messages.iter().filter(|m| m.role == Role::TurnComplete).count();
    assert_eq!(turn_count, 1, "Should have exactly one TurnComplete, got {}", turn_count);

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()));
}

#[test]
fn turn_complete_is_last_when_new_assistant_after_turn_complete() {
    // Simulates a delayed response chunk with a different id arriving after turn complete
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    // New assistant message (different id, not an append) arrives after turn complete
    state.update(Event::AgentResponse { id: "req.1".into(), content: "Delayed".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last even when new assistant message arrives after it: got {:?}", kinds);
}

#[test]
fn turn_complete_is_last_when_error_after_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    // Error arrives after turn complete (but before/during done)
    state.update(Event::AgentError { id: "req.0".into(), message: "Oops".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last even when error arrives after it: got {:?}", kinds);
}

#[test]
fn turn_complete_is_last_when_response_after_done() {
    // Delayed response chunk arrives AFTER AgentDone
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    // Delayed chunk arrives after done
    state.update(Event::AgentResponse { id: "req.0".into(), content: "world".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last even when response arrives after AgentDone: got {:?}", kinds);
}

#[test]
fn turn_complete_is_last_when_thinking_after_done() {
    // Delayed thinking start arrives AFTER AgentDone
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    // Delayed thinking event
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last even when thinking arrives after AgentDone: got {:?}", kinds);
}
