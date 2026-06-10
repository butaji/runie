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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello ".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "First turn".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.session.messages.push(ChatMessage {
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
        "TurnComplete of turn 1 should be before user message of turn 2: got {:?}", kinds);
}

#[test]
fn turn_complete_timestamp_is_max_after_done() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });

    let turn_ts = state.session.messages.iter()
        .find(|m| m.role == Role::TurnComplete)
        .map(|m| m.timestamp)
        .unwrap();
    let max_other_ts = state.session.messages.iter()
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let turn_count = state.session.messages.iter().filter(|m| m.role == Role::TurnComplete).count();
    assert_eq!(turn_count, 1, "Should have exactly one TurnComplete, got {}", turn_count);

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()));
}

#[test]
fn turn_complete_is_last_when_new_assistant_after_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentError { id: "req.0".into(), message: "Oops".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last even when error arrives after it: got {:?}", kinds);
}

#[test]
fn turn_complete_is_last_when_response_after_done() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "world".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last even when response arrives after AgentDone: got {:?}", kinds);
}

#[test]
fn turn_complete_is_last_when_thinking_after_done() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()),
        "TurnComplete must remain last even when thinking arrives after AgentDone: got {:?}", kinds);
}

fn run_turn(state: &mut AppState, id: &str, tool_name: &str, agent_content: &str) {
    state.update(Event::AgentThinking { id: id.into() });
    state.update(Event::AgentThoughtDone { id: id.into() });
    state.update(Event::AgentToolStart { id: id.into(), name: tool_name.into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.3, output: "x".into() });
    state.update(Event::AgentResponse { id: id.into(), content: agent_content.into() });
    state.update(Event::AgentTurnComplete { id: id.into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: id.into() });
}

/// Bug: move_turn_complete_to_end() moved ANY TurnComplete, causing
/// turn 1's TurnComplete to leapfrog turn 2's content.
#[test]
fn turn_complete_order_preserved_across_multiple_turns() {
    let mut state = fresh_state();
    state.streaming = true;
    run_turn(&mut state, "req.0", "ls", "First turn");
    run_turn(&mut state, "req.1", "cat", "Second turn");
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    let turn_positions: Vec<usize> = kinds.iter().enumerate()
        .filter(|(_, k)| *k == "Turn")
        .map(|(i, _)| i)
        .collect();

    assert_eq!(turn_positions.len(), 2, "Expected 2 TurnComplete, got {:?}", kinds);
    let (turn1_pos, turn2_pos) = (turn_positions[0], turn_positions[1]);
    assert!(turn1_pos < turn2_pos,
        "Turn 1 (pos {}) must be before turn 2 (pos {}): got {:?}",
        turn1_pos, turn2_pos, kinds);

    let second_agent_pos = kinds.iter().rposition(|k| k == "Agent").unwrap();
    assert!(turn1_pos < second_agent_pos,
        "Turn 1 must be before turn 2's agent: got {:?}", kinds);
}
