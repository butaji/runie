use crate::event::Event;
use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use crate::model::{AppState, Role};
use crate::ui::LazyCache;

fn fresh_state() -> AppState {
    AppState::default()
}

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
        })
        .filter(|k| k != "Spacer")
        .collect()
}

fn feed_has_turn_complete(state: &AppState) -> bool {
    let feed = LazyCache::feed(state);
    feed.elements
        .iter()
        .any(|e| matches!(e, crate::ui::Element::TurnComplete { .. }))
}

#[test]
fn single_thought_hides_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 0.5,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    assert!(!feed_has_turn_complete(&state));
}

#[test]
fn tool_plus_thought_shows_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.3,
        output: "file1".into(),
    }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Found it".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn tool_only_hides_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "start".into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.3,
        output: "file1".into(),
    }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    assert!(!feed_has_turn_complete(&state));
}

#[test]
fn two_thoughts_shows_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Answer".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 2.0,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn two_tools_shows_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.1,
        output: "a".into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "cat".into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.2,
        output: "b".into(),
    }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 3.0,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn mixed_thought_tool_shows_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.5,
        output: "a".into(),
    }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 1.5,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn zero_actions_hides_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Hello".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 0.1,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    assert!(!feed_has_turn_complete(&state));
}

fn first_turn_events() -> Vec<Event> {
    vec![
        Event::Agent(AgentEvent::Thinking { id: "req.0".into() }),
        Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }),
        Event::Agent(AgentEvent::ToolStart {
            id: "req.0".into(),
            name: "ls".into(),
        }),
        Event::Agent(AgentEvent::ToolEnd {
            duration_secs: 0.5,
            output: "a".into(),
        }),
        Event::Agent(AgentEvent::Response {
            id: "req.0".into(),
            content: "First".into(),
        }),
        Event::Agent(AgentEvent::TurnComplete {
            id: "req.0".into(),
            duration_secs: 1.0,
        }),
        Event::Agent(AgentEvent::Done { id: "req.0".into() }),
    ]
}

fn second_turn_events() -> Vec<Event> {
    vec![
        Event::Agent(AgentEvent::Thinking { id: "req.1".into() }),
        Event::Agent(AgentEvent::ToolStart {
            id: "req.1".into(),
            name: "cat".into(),
        }),
        Event::Agent(AgentEvent::ToolEnd {
            duration_secs: 0.3,
            output: "b".into(),
        }),
        Event::Agent(AgentEvent::Response {
            id: "req.1".into(),
            content: "Second".into(),
        }),
        Event::Agent(AgentEvent::TurnComplete {
            id: "req.1".into(),
            duration_secs: 0.8,
        }),
        Event::Agent(AgentEvent::Done { id: "req.1".into() }),
    ]
}

#[test]
fn second_turn_independent_action_count() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    dispatch(&mut state, &first_turn_events());
    dispatch(&mut state, &second_turn_events());
    state.ensure_fresh();
    let kinds = element_kinds_no_spacer(&state);
    let turn_count = kinds.iter().filter(|k| *k == "Turn").count();
    assert_eq!(
        turn_count, 1,
        "Only turn 1's TurnComplete should be visible; got {:?}",
        kinds
    );
}

#[test]
fn three_mixed_actions_shows_turn_complete() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    }));
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.1,
        output: "a".into(),
    }));
    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 2.0,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    assert!(feed_has_turn_complete(&state));
}

#[test]
fn turn_complete_still_in_session_when_hidden() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Thinking { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::ThoughtDone { id: "req.0".into() }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "Done".into(),
    }));
    state.update(Event::Agent(AgentEvent::TurnComplete {
        id: "req.0".into(),
        duration_secs: 0.5,
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    let turn_msgs = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::TurnComplete)
        .count();
    assert_eq!(turn_msgs, 1);
    assert!(!feed_has_turn_complete(&state));
}
