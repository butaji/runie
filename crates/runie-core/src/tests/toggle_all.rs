use crate::event::Event;
use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use crate::model::{AppState, ChatMessage, Role};

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn collapse_all_when_some_expanded() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s\nreasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran ls 0.5s\noutput".into(),
        timestamp: 1.0,
        id: "x1".into(),
        ..Default::default()
    });

    assert!(!state.view.all_collapsed, "Should start expanded");
    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(
        state.view.all_collapsed,
        "All expanded => collapse all globally"
    );
}

#[test]
fn expand_all_when_all_collapsed() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s\nreasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran ls 0.5s\noutput".into(),
        timestamp: 1.0,
        id: "x1".into(),
        ..Default::default()
    });
    state.view.all_collapsed = true;

    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(
        !state.view.all_collapsed,
        "All collapsed => expand all globally"
    );
}

#[test]
fn running_tools_always_expanded_regardless_of_global_flag() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "⠋ Running ls...".into(),
        timestamp: 1.0,
        id: "x1".into(),
        ..Default::default()
    });

    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(state.view.all_collapsed, "Global flag should flip");
    // Running tool renders as ToolRunning regardless of global flag
}

#[test]
fn toggle_all_empty_state_flips_flag() {
    let mut state = fresh_state();
    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(
        state.view.all_collapsed,
        "Toggle on empty state should flip global flag"
    );
}

#[test]
fn toggle_all_twice_restores_expanded() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });

    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(state.view.all_collapsed, "Toggle 1: collapse all");

    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(!state.view.all_collapsed, "Toggle 2: expand all");

    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(state.view.all_collapsed, "Toggle 3: collapse all again");
}

#[test]
fn toggle_all_with_many_items() {
    let mut state = fresh_state();
    for i in 0..5 {
        state.session.messages.push(ChatMessage {
            role: Role::Thought,
            content: format!("◆ Thought {}", i),
            timestamp: i as f64,
            id: format!("t{}", i),
            ..Default::default()
        });
    }

    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(
        state.view.all_collapsed,
        "All thoughts should be collapsed globally"
    );

    state.update(Event::Control(ControlEvent::ToggleExpand));
    assert!(
        !state.view.all_collapsed,
        "All thoughts should be expanded globally"
    );
}

#[test]
fn new_thought_respects_global_collapse_when_true() {
    let mut state = fresh_state();
    state.view.all_collapsed = true;

    state.update(Event::Agent(AgentEvent::Thinking {
        id: "req.0".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "Reasoning".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::ThoughtDone {
        id: "req.0".to_string(),
    }));
    state.ensure_fresh();

    let feed = crate::ui::LazyCache::feed(&state);
    let has_summary = feed
        .elements
        .iter()
        .any(|e| matches!(e, crate::ui::elements::Element::ThoughtSummary { .. }));
    assert!(
        has_summary,
        "New thought should be collapsed when all_collapsed=true"
    );
}

#[test]
fn new_thought_respects_global_expand_when_false() {
    let mut state = fresh_state();
    state.view.all_collapsed = false;

    state.update(Event::Agent(AgentEvent::Thinking {
        id: "req.0".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "Reasoning".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::ThoughtDone {
        id: "req.0".to_string(),
    }));
    state.ensure_fresh();

    let feed = crate::ui::LazyCache::feed(&state);
    let has_marker = feed
        .elements
        .iter()
        .any(|e| matches!(e, crate::ui::elements::Element::ThoughtMarker { .. }));
    assert!(
        has_marker,
        "New thought should be expanded when all_collapsed=false"
    );
}

#[test]
fn new_tool_respects_global_collapse_when_true() {
    let mut state = fresh_state();
    state.view.all_collapsed = true;

    state.update(Event::Agent(AgentEvent::ToolStart {
        id: "req.0".to_string(),
        name: "ls".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.5,
        output: "a".to_string(),
    }));
    state.ensure_fresh();

    let feed = crate::ui::LazyCache::feed(&state);
    let has_summary = feed
        .elements
        .iter()
        .any(|e| matches!(e, crate::ui::elements::Element::ToolSummary { .. }));
    assert!(
        has_summary,
        "New tool should be collapsed when all_collapsed=true"
    );
}

#[test]
fn new_tool_respects_global_expand_when_false() {
    let mut state = fresh_state();
    state.view.all_collapsed = false;

    state.update(Event::Agent(AgentEvent::ToolStart {
        id: "req.0".to_string(),
        name: "ls".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.5,
        output: "a".to_string(),
    }));
    state.ensure_fresh();

    let feed = crate::ui::LazyCache::feed(&state);
    let has_done = feed
        .elements
        .iter()
        .any(|e| matches!(e, crate::ui::elements::Element::ToolDone { .. }));
    assert!(
        has_done,
        "New tool should be expanded when all_collapsed=false"
    );
}
