use crate::event::Event;
use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use crate::model::AppState;

fn fresh_state() -> AppState {
    AppState::default()
}

fn big_output() -> String {
    (1..=20)
        .map(|i| format!("file{}.txt", i))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_tool_turn(state: &mut AppState, response: &str, tool_output: &str) {
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: response.into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolStart { id: "req.0".into(), name: "ls".into(), input: serde_json::Value::Null }));
    state.update(Event::Agent(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 0.5, output: tool_output.into(),
     }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
}

fn agent_pos(state: &AppState) -> Option<usize> {
    crate::ui::LazyCache::feed(state)
        .elements
        .iter()
        .position(|e| matches!(e, crate::ui::Element::AgentMessage { .. }))
}

fn tool_pos(state: &AppState) -> Option<usize> {
    crate::ui::LazyCache::feed(state)
        .elements
        .iter()
        .position(|e| matches!(e, crate::ui::Element::ToolDone { .. }))
}

fn thought_pos(state: &AppState) -> Option<usize> {
    crate::ui::LazyCache::feed(state)
        .elements
        .iter()
        .position(|e| matches!(e, crate::ui::Element::ThoughtMarker { .. }))
}

fn agent_turn_complete_kinds(state: &AppState) -> Vec<&'static str> {
    crate::ui::LazyCache::feed(state)
        .elements
        .iter()
        .map(|e| match e {
            crate::ui::Element::AgentMessage { .. } => "A",
            crate::ui::Element::TurnComplete { .. } => "T",
            crate::ui::Element::Spacer { .. } => "S",
            _ => "?",
        })
        .collect()
}

/// The bug: when agent response arrives before tool (mock provider),
/// large tool output pushes the agent response above the viewport.
/// After finish_turn, the final agent response must be AFTER tools.
#[test]
fn final_agent_after_tools_when_turn_completes() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    run_tool_turn(&mut state, "Done!", &big_output());
    let (a, t) = (agent_pos(&state), tool_pos(&state));
    assert!(a.is_some(), "Agent message must exist");
    assert!(t.is_some(), "Tool message must exist");
    assert!(
        a.unwrap() > t.unwrap(),
        "Final agent must appear AFTER tool"
    );
}

#[test]
fn final_agent_visible_when_tool_overflows() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    run_tool_turn(&mut state, "Done!", &big_output());
    state.view.scroll = 0;
    let region = crate::tests::visible_helper::compute_viewport(&state, 5);
    let has_agent = region.elements.iter().any(
        |e| matches!(e, crate::ui::Element::AgentMessage { content, .. } if content == "Done!"),
    );
    assert!(has_agent, "Final agent 'Done!' must be visible at bottom");
}

#[test]
fn agent_before_tool_preserved_during_turn() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done!".into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolStart { id: "req.0".into(), name: "ls".into(), input: serde_json::Value::Null }));
    state.update(Event::Agent(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 0.5, output: "a".into(),
     }));
    state.ensure_fresh();
    assert!(agent_pos(&state).is_some() && tool_pos(&state).is_some());
}

#[test]
fn no_reorder_when_no_tools() {
    let mut state = fresh_state();
    state.agent.streaming = true;

    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ToolStart { id: "req.0".into(), name: "ls".into(), input: serde_json::Value::Null }));
    state.update(Event::Agent(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 0.5, output: "a".into(),
     }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Hello".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    let kinds = agent_turn_complete_kinds(&state);
    assert!(
        kinds.iter().position(|&k| k == "A").unwrap()
            < kinds.iter().position(|&k| k == "T").unwrap(),
        "Agent should be before TurnComplete: got {:?}",
        kinds
    );
}

#[test]
fn thought_stays_before_tool_after_reorder() {
    let mut state = fresh_state();
    state.agent.streaming = true;

    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "I'll list files.\nTOOL:list_dir:.".into(),
    }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    run_tool_turn(&mut state, "Done!", "file1");

    let (t, o, a) = (thought_pos(&state), tool_pos(&state), agent_pos(&state));
    assert!(t.is_some() && o.is_some() && a.is_some());
    assert!(t.unwrap() < o.unwrap(), "Thought must be before tool");
    assert!(o.unwrap() < a.unwrap(), "Agent must be after tool");
}
