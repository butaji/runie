use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;
use crate::ui::LazyCache;
use crate::ui::elements::Element;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn collapsed_thought_stays_collapsed_after_agent_response() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "I'll list files.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    // Collapse the thought
    state.update(Event::ToggleExpand);
    let thought_id = state.messages.iter().find(|m| m.role == Role::Thought).unwrap().id.clone();
    assert!(state.collapsed.contains(&thought_id));

    // New agent response arrives
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Here they are.".to_string() });
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let has_summary = feed.elements.iter().any(|e| matches!(e, Element::ThoughtSummary { .. }));
    assert!(has_summary, "Thought should stay collapsed after new response arrives");
    let has_marker = feed.elements.iter().any(|e| matches!(e, Element::ThoughtMarker { .. }));
    assert!(!has_marker, "ThoughtMarker should not appear when collapsed");
}

#[test]
fn collapsed_tool_stays_collapsed_after_agent_response() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_dir".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1".to_string() });

    // Collapse the tool
    state.update(Event::ToggleExpand);
    let tool_id = state.messages.iter().find(|m| m.role == Role::Tool).unwrap().id.clone();
    assert!(state.collapsed.contains(&tool_id));

    // New agent response arrives
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Done.".to_string() });
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let has_summary = feed.elements.iter().any(|e| matches!(e, Element::ToolSummary { .. }));
    assert!(has_summary, "Tool should stay collapsed after new response arrives");
    let has_done = feed.elements.iter().any(|e| matches!(e, Element::ToolDone { .. }));
    assert!(!has_done, "ToolDone should not appear when collapsed");
}

#[test]
fn collapsed_thought_stays_collapsed_after_second_thought() {
    let mut state = fresh_state();
    state.streaming = true;

    // First thought
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    // Collapse first thought
    state.update(Event::ToggleExpand);
    let first_id = state.messages.iter().find(|m| m.role == Role::Thought).unwrap().id.clone();
    assert!(state.collapsed.contains(&first_id));

    // Second thought
    state.update(Event::AgentThinking { id: "req.1".to_string() });
    state.update(Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.1".to_string() });
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtSummary { .. })).collect();
    assert_eq!(summaries.len(), 1, "Only first thought should be collapsed");
    let markers: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ThoughtMarker { .. })).collect();
    assert_eq!(markers.len(), 1, "Second thought should be expanded");
}

#[test]
fn collapsed_tool_stays_collapsed_after_second_tool() {
    let mut state = fresh_state();
    state.streaming = true;

    // First tool
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "ls".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".to_string() });

    // Collapse first tool
    state.update(Event::ToggleExpand);
    let first_id = state.messages.iter().find(|m| m.role == Role::Tool).unwrap().id.clone();
    assert!(state.collapsed.contains(&first_id));

    // Second tool
    state.update(Event::AgentToolStart { id: "req.1".to_string(), name: "cat".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.3, output: "b".to_string() });
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let summaries: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ToolSummary { .. })).collect();
    assert_eq!(summaries.len(), 1, "Only first tool should be collapsed");
    let dones: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::ToolDone { .. })).collect();
    assert_eq!(dones.len(), 1, "Second tool should be expanded");
}

#[test]
fn toggle_most_recent_after_new_items() {
    let mut state = fresh_state();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    // Toggle collapses first thought
    state.update(Event::ToggleExpand);
    let first_id = state.messages.iter().find(|m| m.role == Role::Thought).unwrap().id.clone();
    assert!(state.collapsed.contains(&first_id));

    // New thought arrives (expanded by default)
    state.update(Event::AgentThinking { id: "req.1".to_string() });
    state.update(Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.1".to_string() });

    // Toggle again — should toggle the NEWEST thought (second one)
    state.update(Event::ToggleExpand);
    let second_id = state.messages.iter().rfind(|m| m.role == Role::Thought).unwrap().id.clone();
    assert!(state.collapsed.contains(&second_id), "Toggle should collapse most recent thought");
    assert!(state.collapsed.contains(&first_id), "First thought should stay collapsed");
}

#[test]
fn expand_then_collapse_then_expand_same_thought() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.2s\nline1\nline2".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });

    // Toggle 1: collapse
    state.update(Event::ToggleExpand);
    assert!(state.collapsed.contains("t1"));

    // Toggle 2: expand
    state.update(Event::ToggleExpand);
    assert!(!state.collapsed.contains("t1"));

    // Toggle 3: collapse again
    state.update(Event::ToggleExpand);
    assert!(state.collapsed.contains("t1"));

    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let summary = feed.elements.iter().find_map(|e| match e {
        Element::ThoughtSummary { .. } => Some(()),
        _ => None,
    });
    assert!(summary.is_some(), "After 3 toggles thought should be collapsed");
}

#[test]
fn running_tool_ignored_by_toggle() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_dir".to_string() });

    // Try to toggle while tool is still running
    state.update(Event::ToggleExpand);
    assert!(state.collapsed.is_empty(), "Running tool should be ignored by toggle");
}

#[test]
fn reset_clears_all_collapsed() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Thought".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran ls 0.5s".into(),
        timestamp: 0.0,
        id: "x1".into(),
    });
    state.collapsed.insert("t1".into());
    state.collapsed.insert("x1".into());

    state.update(Event::Reset);
    assert!(state.collapsed.is_empty(), "Reset should clear collapsed set");
}

#[test]
fn toggle_does_not_affect_user_or_assistant_messages() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "Hello".into(),
        timestamp: 0.0,
        id: "u1".into(),
    });
    state.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "Hi".into(),
        timestamp: 0.0,
        id: "a1".into(),
    });

    state.update(Event::ToggleExpand);
    assert!(state.collapsed.is_empty(), "Toggle should not affect user/assistant messages");
}

#[test]
fn cache_rebuilds_correctly_after_toggle_and_new_items() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s\nReasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran ls 0.5s\nfile1".into(),
        timestamp: 1.0,
        id: "x1".into(),
    });
    state.ensure_fresh();

    // Collapse both
    state.update(Event::ToggleExpand); // toggles x1 (most recent)
    state.update(Event::ToggleExpand); // toggles x1 again (expands)
    state.update(Event::ToggleExpand); // toggles x1 again (collapses)
    // Now x1 is collapsed, but how to collapse t1?
    // t1 is older, so toggle always picks most recent. We need to manually insert.
    state.collapsed.insert("t1".into());

    state.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "Done".into(),
        timestamp: 2.0,
        id: "a1".into(),
    });
    state.messages_changed();
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let elements: Vec<_> = feed.elements.iter().map(|e| match e {
        Element::ThoughtSummary { .. } => "TS",
        Element::ThoughtMarker { .. } => "TM",
        Element::ToolSummary { .. } => "XS",
        Element::ToolDone { .. } => "XD",
        Element::AgentMessage { .. } => "AM",
        Element::Spacer => "S",
        _ => "?",
    }).collect();

    assert!(elements.contains(&"TS"), "Thought should be collapsed: {:?}", elements);
    assert!(elements.contains(&"XS"), "Tool should be collapsed: {:?}", elements);
    assert!(elements.contains(&"AM"), "Assistant message should be present: {:?}", elements);
}
