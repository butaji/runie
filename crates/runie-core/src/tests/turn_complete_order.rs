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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "First turn".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    // User sends next message
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
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 }); // duplicate
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let turn_count = state.session.messages.iter().filter(|m| m.role == Role::TurnComplete).count();
    assert_eq!(turn_count, 1, "Should have exactly one TurnComplete, got {}", turn_count);

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds.last(), Some(&"Turn".to_string()));
}

#[test]
fn turn_complete_is_last_when_new_assistant_after_turn_complete() {
    // Simulates a delayed response chunk with a different id arriving after turn complete
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
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
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
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

// ─── Conditional visibility: TurnComplete shown only when >1 think/run actions ───────

fn feed_has_turn_complete(state: &AppState) -> bool {
    let feed = LazyCache::feed(state);
    feed.elements.iter().any(|e| matches!(e, crate::ui::Element::TurnComplete { .. }))
}

/// Single thought → TurnComplete hidden (only 1 action = trivial turn)
#[test]
fn single_thought_hides_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 0.5 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    assert!(!feed_has_turn_complete(&state),
        "TurnComplete should be hidden for single-thought turn: {:?}", element_kinds_no_spacer(&state));
}

/// Tool + Thought = 2 actions → TurnComplete shown
#[test]
fn tool_plus_thought_shows_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.3, output: "file1".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Found it".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    // 1 thought + 1 tool = count 2 → TurnComplete shown
    assert!(feed_has_turn_complete(&state),
        "TurnComplete should be visible when tool + thought = 2 actions: {:?}", element_kinds_no_spacer(&state));
}

/// Single tool with NO thought → TurnComplete hidden (count = 1)
#[test]
fn tool_only_hides_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "start".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.3, output: "file1".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    // 1 tool only = count 1 → hidden
    assert!(!feed_has_turn_complete(&state),
        "TurnComplete should be hidden for single-tool (no thought): {:?}", element_kinds_no_spacer(&state));
}

/// Two thoughts → TurnComplete shown
#[test]
fn two_thoughts_shows_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Answer".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 2.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    assert!(feed_has_turn_complete(&state),
        "TurnComplete should be visible with 2 thoughts: {:?}", element_kinds_no_spacer(&state));
}

/// Two tools → TurnComplete shown
#[test]
fn two_tools_shows_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.1, output: "a".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "cat".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.2, output: "b".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 3.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    assert!(feed_has_turn_complete(&state),
        "TurnComplete should be visible with 2 tools: {:?}", element_kinds_no_spacer(&state));
}

/// Thought + Tool → TurnComplete shown (2 actions)
#[test]
fn mixed_thought_tool_shows_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.5 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    assert!(feed_has_turn_complete(&state),
        "TurnComplete should be visible with thought + tool: {:?}", element_kinds_no_spacer(&state));
}

/// Zero actions (just response) → TurnComplete hidden
#[test]
fn zero_actions_hides_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    // No thinking, no thought, no tools — just a direct response
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 0.1 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    assert!(!feed_has_turn_complete(&state),
        "TurnComplete should be hidden with 0 actions: {:?}", element_kinds_no_spacer(&state));
}

/// Two turns: turn 1 has 2 actions → show, turn 2 has 1 action → hide
#[test]
fn second_turn_independent_action_count() {
    let mut state = fresh_state();
    state.streaming = true;

    // Turn 1: thought + tool = 2 actions → show
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "First".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });

    // Turn 2: only tool (no thought) = 1 action → hide
    state.update(Event::AgentThinking { id: "req.1".into() });
    // Note: no AgentThoughtDone → no thought message created
    state.update(Event::AgentToolStart { id: "req.1".into(), name: "cat".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.3, output: "b".into() });
    state.update(Event::AgentResponse { id: "req.1".into(), content: "Second".into() });
    state.update(Event::AgentTurnComplete { id: "req.1".into(), duration_secs: 0.8 });
    state.update(Event::AgentDone { id: "req.1".into() });

    state.ensure_fresh();
    let kinds = element_kinds_no_spacer(&state);
    let turn_count = kinds.iter().filter(|k| *k == "Turn").count();
    assert_eq!(turn_count, 1,
        "Only turn 1's TurnComplete should be visible; got {:?}", kinds);
}

/// Turn with 3 actions (thought + tool + thought) → show
#[test]
fn three_mixed_actions_shows_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.1, output: "a".into() });
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 2.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    assert!(feed_has_turn_complete(&state),
        "TurnComplete should be visible with 3 actions: {:?}", element_kinds_no_spacer(&state));
}

/// TurnComplete still exists in session messages even when hidden from feed
#[test]
fn turn_complete_still_in_session_when_hidden() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 0.5 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    // TurnComplete must still be in session (used for ordering, markdown export, etc.)
    let turn_msgs = state.session.messages.iter().filter(|m| m.role == Role::TurnComplete).count();
    assert_eq!(turn_msgs, 1, "TurnComplete must still exist in session messages");

    // But hidden from feed
    assert!(!feed_has_turn_complete(&state));
}
