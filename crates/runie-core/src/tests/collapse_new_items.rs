use crate::event::Event;
use crate::model::{AppState, ChatMessage, Role};
use crate::ui::elements::Element;
use crate::ui::LazyCache;

fn fresh_state() -> AppState {
    AppState::default()
}

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

#[test]
fn global_collapse_persists_after_agent_response() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "I'll list files.".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Here they are.".to_string(),
    });
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let has_summary = feed
        .elements
        .iter()
        .any(|e| matches!(e, Element::ThoughtSummary { .. }));
    assert!(
        has_summary,
        "Thought should stay collapsed after new response arrives"
    );
    let has_marker = feed
        .elements
        .iter()
        .any(|e| matches!(e, Element::ThoughtMarker { .. }));
    assert!(
        !has_marker,
        "ThoughtMarker should not appear when globally collapsed"
    );
}

#[test]
fn global_collapse_persists_after_second_thought() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    dispatch(
        &mut state,
        &[
            Event::AgentThinking { id: "req.0".into() },
            Event::AgentResponse { id: "req.0".into(), content: "A".into() },
            Event::AgentThoughtDone { id: "req.0".into() },
        ],
    );
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);
    dispatch(
        &mut state,
        &[
            Event::AgentThinking { id: "req.1".into() },
            Event::AgentResponse { id: "req.1".into(), content: "B".into() },
            Event::AgentThoughtDone { id: "req.1".into() },
        ],
    );
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtSummary { .. })).collect();
    assert_eq!(summaries.len(), 2, "BOTH thoughts should be collapsed with global flag");
    let markers: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtMarker { .. })).collect();
    assert_eq!(markers.len(), 0, "No thoughts should be expanded");
}

#[test]
fn global_collapse_persists_after_second_tool() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "ls".to_string(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output: "a".to_string(),
    });
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);
    state.update(Event::AgentToolStart {
        id: "req.1".to_string(),
        name: "cat".to_string(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.3,
        output: "b".to_string(),
    });
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ToolSummary { .. }))
        .collect();
    assert_eq!(
        summaries.len(),
        2,
        "BOTH tools should be collapsed with global flag"
    );
    let dones: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ToolDone { .. }))
        .collect();
    assert_eq!(dones.len(), 0, "No tools should be expanded");
}

#[test]
fn new_thought_respects_global_collapse_flag() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "A".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);
    state.update(Event::AgentThinking {
        id: "req.1".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.1".to_string(),
        content: "B".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.1".to_string(),
    });
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ThoughtSummary { .. }))
        .collect();
    assert_eq!(
        summaries.len(),
        2,
        "New thought should respect global collapse"
    );
}

#[test]
fn new_tool_respects_global_collapse_flag() {
    let mut state = fresh_state();
    state.agent.streaming = true;

    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "ls".to_string(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output: "a".to_string(),
    });

    // Collapse all
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);

    // New tool arrives while globally collapsed
    state.update(Event::AgentToolStart {
        id: "req.1".to_string(),
        name: "cat".to_string(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.3,
        output: "b".to_string(),
    });
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, Element::ToolSummary { .. }))
        .collect();
    assert_eq!(
        summaries.len(),
        2,
        "New tool should respect global collapse"
    );
}

#[test]
fn expand_then_collapse_then_expand_same_state() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.2s\nline1\nline2".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });

    // Toggle 1: collapse all
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);

    // Toggle 2: expand all
    state.update(Event::ToggleExpand);
    assert!(!state.view.all_collapsed);

    // Toggle 3: collapse all again
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);

    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let summary = feed.elements.iter().find_map(|e| match e {
        Element::ThoughtSummary { .. } => Some(()),
        _ => None,
    });
    assert!(
        summary.is_some(),
        "After 3 toggles thought should be collapsed"
    );
}

#[test]
fn running_tool_ignored_by_global_toggle() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "list_dir".to_string(),
    });

    // Toggle while tool is still running
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed, "Toggle should still flip global flag");

    // But running tool renders as ToolRunning, not ToolSummary
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let has_running = feed
        .elements
        .iter()
        .any(|e| matches!(e, Element::ToolRunning { .. }));
    assert!(
        has_running,
        "Running tool should still show as running regardless of global flag"
    );
}

#[test]
fn reset_clears_global_collapse() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Thought".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.view.all_collapsed = true;

    state.update(Event::Reset);
    assert!(
        !state.view.all_collapsed,
        "Reset should clear global collapse flag"
    );
}

#[test]
fn global_toggle_does_not_affect_user_or_assistant_messages() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "Hello".into(),
        timestamp: 0.0,
        id: "u1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "Hi".into(),
        timestamp: 0.0,
        id: "a1".into(),
        ..Default::default()
    });

    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "Global flag should flip even with no thoughts/tools"
    );
}

#[test]
fn cache_rebuilds_correctly_with_global_collapse_and_new_items() {
    let mut state = fresh_state();
    add_thought_and_tool(&mut state);
    state.ensure_fresh();

    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);

    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "Done".into(),
        timestamp: 2.0,
        id: "a1".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    verify_collapsed_elements(&state);
}

fn add_thought_and_tool(state: &mut AppState) {
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s\nReasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran ls 0.5s\nfile1".into(),
        timestamp: 1.0,
        id: "x1".into(),
        ..Default::default()
    });
}

fn verify_collapsed_elements(state: &AppState) {
    let feed = LazyCache::feed(state);
    let elements: Vec<_> = feed
        .elements
        .iter()
        .map(|e| match e {
            Element::ThoughtSummary { .. } => "TS",
            Element::ThoughtMarker { .. } => "TM",
            Element::ToolSummary { .. } => "XS",
            Element::ToolDone { .. } => "XD",
            Element::AgentMessage { .. } => "AM",
            Element::Spacer { .. } => "S",
            _ => "?",
        })
        .collect();

    assert!(
        elements.contains(&"TS"),
        "Thought should be collapsed: {:?}",
        elements
    );
    assert!(
        elements.contains(&"XS"),
        "Tool should be collapsed: {:?}",
        elements
    );
    assert!(
        elements.contains(&"AM"),
        "Assistant message should be present: {:?}",
        elements
    );
}
