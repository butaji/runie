use crate::event::Event;
use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use crate::model::AppState;
use crate::ui::elements::Element;
use crate::ui::LazyCache;

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

fn first_turn_before_collapse() -> Vec<Event> {
    vec![
        Event::Agent(AgentEvent::Thinking { id: "req.0".into() }),
        Event::Agent(AgentEvent::Response {
            id: "req.0".into(),
            content: "I'll list files.\n".into(),
        }),
        Event::Agent(AgentEvent::Response {
            id: "req.0".into(),
            content: "TOOL:list_dir:.".into(),
        }),
        Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }),
    ]
}

fn first_turn_after_collapse() -> Vec<Event> {
    vec![
        Event::Agent(AgentEvent::ToolStart { id: "req.0".into(), name: "list_dir".into(), input: serde_json::Value::Null }),
        Event::Agent(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 0.5, output: "file1\nfile2".into(),
         }),
        Event::Agent(AgentEvent::Response {
            id: "req.0".into(),
            content: "Done.".into(),
        }),
        Event::Agent(AgentEvent::TurnComplete {
            id: "req.0".into(),
            duration_secs: 1.0,
        }),
        Event::Agent(AgentEvent::Done { id: "req.0".into() }),
    ]
}

fn turn_events(id: &str, content: &str, tool: &str, output: &str) -> Vec<Event> {
    vec![
        Event::Agent(AgentEvent::Thinking { id: id.into() }),
        Event::Agent(AgentEvent::Response {
            id: id.into(),
            content: content.into(),
        }),
        Event::Agent(AgentEvent::Response {
            id: id.into(),
            content: format!("TOOL:{}.", tool),
        }),
        Event::Agent(AgentEvent::ThoughtDone { id: id.into() }),
        Event::Agent(AgentEvent::ToolStart { id: id.into(), name: tool.into(), input: serde_json::Value::Null }),
        Event::Agent(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 0.5, output: output.into(),
         }),
        Event::Agent(AgentEvent::Done { id: id.into() }),
    ]
}

fn multiple_tool_first_half() -> Vec<Event> {
    vec![
        Event::Agent(AgentEvent::Thinking { id: "req.0".into() }),
        Event::Agent(AgentEvent::Response {
            id: "req.0".into(),
            content: "I'll do two things.\n".into(),
        }),
        Event::Agent(AgentEvent::Response {
            id: "req.0".into(),
            content: "TOOL:list_dir:.".into(),
        }),
        Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }),
        Event::Agent(AgentEvent::ToolStart { id: "req.0".into(), name: "ls".into(), input: serde_json::Value::Null }),
        Event::Agent(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 0.5, output: "a".into(),
         }),
    ]
}

fn multiple_tool_second_half() -> Vec<Event> {
    vec![
        Event::Agent(AgentEvent::Thinking { id: "req.0".into() }),
        Event::Agent(AgentEvent::Response {
            id: "req.0".into(),
            content: "Now grep.\n".into(),
        }),
        Event::Agent(AgentEvent::Response {
            id: "req.0".into(),
            content: "TOOL:grep:fn main:.".into(),
        }),
        Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }),
        Event::Agent(AgentEvent::ToolStart { id: "req.0".into(), name: "grep".into(), input: serde_json::Value::Null }),
        Event::Agent(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 0.3, output: "result".into(),
         }),
        Event::Agent(AgentEvent::Done { id: "req.0".into() }),
    ]
}

fn single_thought_events(id: &str, reasoning: &str, output: &str) -> Vec<Event> {
    vec![
        Event::Agent(AgentEvent::Thinking { id: id.into() }),
        Event::Agent(AgentEvent::Response {
            id: id.into(),
            content: format!("{}\n", reasoning),
        }),
        Event::Agent(AgentEvent::Response {
            id: id.into(),
            content: "TOOL:ls.".into(),
        }),
        Event::Agent(AgentEvent::ThoughtDone { id: id.into() }),
        Event::Agent(AgentEvent::ToolStart { id: id.into(), name: "ls".into(), input: serde_json::Value::Null }),
        Event::Agent(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 0.1, output: output.into(),
         }),
        Event::Agent(AgentEvent::Done { id: id.into() }),
    ]
}

fn assert_summary_count(state: &AppState, expected: usize, msg: &str) {
    let feed = LazyCache::feed(state);
    let summaries: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ThoughtSummary { .. }))
        .collect();
    assert_eq!(summaries.len(), expected, "{}", msg);
}

#[test]
fn global_collapse_persists_through_rapid_events() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    dispatch(&mut state, &first_turn_before_collapse());
    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(state.view.all_collapsed, "Global flag should be set");
    dispatch(&mut state, &first_turn_after_collapse());
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let thoughts: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ThoughtSummary { .. }))
        .collect();
    assert_eq!(thoughts.len(), 1, "Thought should be collapsed");
    let tools: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ToolSummary { .. }))
        .collect();
    assert_eq!(tools.len(), 1, "Tool should also be collapsed");
}

#[test]
fn global_collapse_persists_when_second_turn_starts() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    dispatch(&mut state, &turn_events("req.0", "A", "ls", "a"));
    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(state.view.all_collapsed);
    dispatch(&mut state, &turn_events("req.1", "B", "grep", "b"));
    state.ensure_fresh();

    assert_summary_count(&state, 2, "Both turns' thoughts should be collapsed");
    let feed = LazyCache::feed(&state);
    let tools: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ToolSummary { .. }))
        .collect();
    assert_eq!(tools.len(), 2, "Both turns' tools should be collapsed");
}

#[test]
fn global_collapse_persists_through_multiple_tools_in_one_turn() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    dispatch(&mut state, &multiple_tool_first_half());
    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(state.view.all_collapsed);
    dispatch(&mut state, &multiple_tool_second_half());
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let tools: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ToolSummary { .. }))
        .collect();
    assert_eq!(tools.len(), 2, "BOTH tools should be collapsed");
}

#[test]
fn multiple_thoughts_all_follow_global_flag() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    for n in 0..3 {
        dispatch(
            &mut state,
            &single_thought_events(
                &format!("req.{}", n),
                &format!("reasoning {}", n),
                &format!("out{}", n),
            ),
        );
    }
    state.update(Event::Control(ControlEvent::ToggleExpand));
    state.ensure_fresh();

    assert_summary_count(&state, 3, "All three thoughts should be collapsed");
    let feed = LazyCache::feed(&state);
    let markers: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ThoughtMarker { .. }))
        .collect();
    assert_eq!(markers.len(), 0, "No thoughts should be expanded");
}
