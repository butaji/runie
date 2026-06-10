use crate::model::{AppState, Role};
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

fn feed_has_turn_complete(state: &AppState) -> bool {
    let feed = LazyCache::feed(state);
    feed.elements.iter().any(|e| matches!(e, crate::ui::Element::TurnComplete { .. }))
}

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
    assert!(!feed_has_turn_complete(&state));
}

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
    assert!(feed_has_turn_complete(&state));
}

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
    assert!(!feed_has_turn_complete(&state));
}

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
    assert!(feed_has_turn_complete(&state));
}

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
    assert!(feed_has_turn_complete(&state));
}

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
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn zero_actions_hides_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 0.1 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();
    assert!(!feed_has_turn_complete(&state));
}

#[test]
fn second_turn_independent_action_count() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "First".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });

    state.update(Event::AgentThinking { id: "req.1".into() });
    state.update(Event::AgentToolStart { id: "req.1".into(), name: "cat".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.3, output: "b".into() });
    state.update(Event::AgentResponse { id: "req.1".into(), content: "Second".into() });
    state.update(Event::AgentTurnComplete { id: "req.1".into(), duration_secs: 0.8 });
    state.update(Event::AgentDone { id: "req.1".into() });

    state.ensure_fresh();
    let kinds = element_kinds_no_spacer(&state);
    let turn_count = kinds.iter().filter(|k| *k == "Turn").count();
    assert_eq!(turn_count, 1, "Only turn 1's TurnComplete should be visible; got {:?}", kinds);
}

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
    assert!(feed_has_turn_complete(&state));
}

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
    let turn_msgs = state.session.messages.iter().filter(|m| m.role == Role::TurnComplete).count();
    assert_eq!(turn_msgs, 1);
    assert!(!feed_has_turn_complete(&state));
}
