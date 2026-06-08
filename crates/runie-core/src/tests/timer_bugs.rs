use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;
use crate::ui::LazyCache;

fn fresh_state() -> AppState {
    AppState::default()
}

fn element_kinds(state: &AppState) -> Vec<String> {
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
    }).collect()
}

#[test]
fn completed_tool_with_running_in_name_renders_as_tool_done() {
    let mut state = fresh_state();
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "listRunningProcs".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "pid 123".into() });
    state.ensure_fresh();

    let k = element_kinds(&state);
    assert!(
        k.iter().any(|x| x == "ToolDone"),
        "Completed tool should render as ToolDone, even if name contains 'Running'. Got: {:?}", k
    );
    assert!(
        !k.iter().any(|x| x == "ToolRun"),
        "Completed tool should NOT render as ToolRun. Got: {:?}", k
    );
}

#[test]
fn completed_tool_running_check_does_not_show_timer() {
    let mut state = fresh_state();
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "isRunning".into() });
    state.update(Event::AgentToolEnd { duration_secs: 1.2, output: "yes".into() });
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    for elem in &feed.elements {
        if let crate::ui::Element::ToolRunning { name, .. } = elem {
            panic!("Should not have ToolRunning for completed tool '{}', but got: {:?}", name, elem);
        }
    }
}

#[test]
fn finish_turn_does_not_clear_next_turns_thinking() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "T1".into() });
    state.update(Event::AgentThinking { id: "req.1".into() });
    state.update(Event::AgentDone { id: "req.0".into() });

    assert!(
        state.thinking_started_at.is_some(),
        "finish_turn must NOT clear thinking_started_at for the next turn"
    );
}

#[test]
fn next_turn_thinking_shows_after_previous_turn_complete() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "First".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentThinking { id: "req.1".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let k: Vec<_> = element_kinds(&state).into_iter().filter(|x| x != "Spacer").collect();
    let turn_pos = k.iter().position(|x| x == "Turn");
    let thinking_pos = k.iter().position(|x| x == "Thinking");
    assert!(turn_pos.is_some(), "TurnComplete should exist");
    assert!(thinking_pos.is_some(), "Thinking for turn 2 should exist");
    assert!(
        turn_pos.unwrap() < thinking_pos.unwrap(),
        "TurnComplete of turn 1 must be before Thinking of turn 2. Got: {:?}", k
    );
}

#[test]
fn thinking_indicator_gone_after_thought_done() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let k: Vec<_> = element_kinds(&state).into_iter().filter(|x| x != "Spacer").collect();
    assert!(
        !k.iter().any(|x| x == "Thinking"),
        "Thinking indicator should be gone after thought done. Got: {:?}", k
    );
}

#[test]
fn only_one_turn_complete_after_done() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();

    let k: Vec<_> = element_kinds(&state).into_iter().filter(|x| x != "Spacer").collect();
    let turn_count = k.iter().filter(|x| *x == "Turn").count();
    assert_eq!(turn_count, 1, "Should have exactly one TurnComplete. Got: {:?}", k);
}

#[test]
fn turn_complete_timestamp_monotonically_increases() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "A".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });

    let ts1 = state.messages.iter()
        .find(|m| m.role == Role::TurnComplete)
        .map(|m| m.timestamp)
        .unwrap();

    state.update(Event::AgentResponse { id: "req.0".into(), content: "B".into() });

    let ts2 = state.messages.iter()
        .find(|m| m.role == Role::TurnComplete)
        .map(|m| m.timestamp)
        .unwrap();

    assert!(
        ts2 >= ts1,
        "TurnComplete timestamp must not regress: {} -> {}", ts1, ts2
    );
}
