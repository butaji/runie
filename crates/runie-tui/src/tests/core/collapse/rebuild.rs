//! rebuild tests.

use runie_core::event::{AgentEvent, ControlEvent};
use runie_core::model::{AppState, ChatMessage, Role};
use runie_core::ui::elements::Element;
use runie_core::ui::LazyCache;
fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn toggle_expand_affects_all_items() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "older thought".into(),
        timestamp: 0.0,
        id: "old".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ list_files 0.5s".into(),
        timestamp: 1.0,
        id: "new".into(),
        ..Default::default()
    });
    state.update(ControlEvent::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "Toggle should collapse ALL thoughts and tools globally"
    );
}

#[test]
fn toggle_thought_rebuilds_cache() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning\nline two".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.ensure_fresh();
    let before = state.view.elements_cache().to_vec();
    assert!(before
        .iter()
        .any(|e| matches!(e, Element::ThoughtMarker { .. })));

    state.update(ControlEvent::ToggleExpand);
    state.ensure_fresh();
    let after = state.view.elements_cache().to_vec();
    assert!(
        after
            .iter()
            .any(|e| matches!(e, Element::ThoughtSummary { .. })),
        "Cache should rebuild to ThoughtSummary after toggle"
    );
}

#[test]
fn toggle_thought_twice_restores_cache() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(ControlEvent::ToggleExpand);
    state.ensure_fresh();
    state.update(ControlEvent::ToggleExpand);
    state.ensure_fresh();
    let cache = state.view.elements_cache().to_vec();
    assert!(
        cache
            .iter()
            .any(|e| matches!(e, Element::ThoughtMarker { .. })),
        "Cache should restore ThoughtMarker after second toggle"
    );
}

#[test]
fn toggle_tool_rebuilds_cache() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.ensure_fresh();
    let before = state.view.elements_cache().to_vec();
    assert!(before.iter().any(|e| matches!(e, Element::ToolDone { .. })));

    state.update(ControlEvent::ToggleExpand);
    state.ensure_fresh();
    let after = state.view.elements_cache().to_vec();
    assert!(
        after
            .iter()
            .any(|e| matches!(e, Element::ToolSummary { .. })),
        "Cache should rebuild to ToolSummary after toggle"
    );
}

#[test]
fn toggle_tool_twice_restores_cache() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(ControlEvent::ToggleExpand);
    state.ensure_fresh();
    state.update(ControlEvent::ToggleExpand);
    state.ensure_fresh();
    let cache = state.view.elements_cache().to_vec();
    assert!(
        cache.iter().any(|e| matches!(e, Element::ToolDone { .. })),
        "Cache should restore ToolDone after second toggle"
    );
}

#[test]
fn thought_captures_assistant_reasoning() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "I'll list the files.\n".to_string(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "TOOL:list_dir:.".to_string(),
    });
    state.update(AgentEvent::ThoughtDone {
        id: "req.0".to_string(),
    });

    let thought = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Thought)
        .unwrap();
    assert!(
        thought.content.contains("I'll list the files."),
        "Thought should capture reasoning: {}",
        thought.content
    );
    assert!(
        !thought.content.contains("TOOL:"),
        "Thought should have tool markers stripped"
    );
}

#[test]
fn assistant_preserved_when_no_tools() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "Here is the answer.".to_string(),
    });
    state.update(AgentEvent::ThoughtDone {
        id: "req.0".to_string(),
    });

    let assistants: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .collect();
    assert_eq!(
        assistants.len(),
        1,
        "Assistant should be preserved when no tools"
    );
    assert_eq!(assistants[0].content, "Here is the answer.");
}

#[test]
fn tool_stores_output() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::ToolStart {
        id: "req.0".to_string(),
        name: "list_dir".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "file1\nfile2".to_string(),
    });

    let tool = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Tool)
        .unwrap();
    assert!(
        tool.content.contains("file1"),
        "Tool should store output: {}",
        tool.content
    );
    assert!(
        tool.content.contains("file2"),
        "Tool should store output: {}",
        tool.content
    );
}

#[test]
fn collapsed_thought_hides_reasoning() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.2s\nI'll list the files.".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.view.all_collapsed = true;
    let feed = LazyCache::feed(&state);

    let summary = feed.elements.iter().find_map(|e| match e {
        Element::ThoughtSummary { content, .. } => Some(content.as_str()),
        _ => None,
    });
    assert!(summary.is_some());
    assert!(
        !summary.unwrap().contains("I'll list"),
        "Collapsed thought should hide reasoning"
    );
}

#[test]
fn expanded_thought_shows_reasoning() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.2s\nI'll list the files.".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    let feed = LazyCache::feed(&state);

    let marker = feed.elements.iter().find_map(|e| match e {
        Element::ThoughtMarker { content, .. } => Some(content.as_str()),
        _ => None,
    });
    assert!(marker.is_some());
    assert!(
        marker.unwrap().contains("I'll list"),
        "Expanded thought should show reasoning"
    );
}

#[test]
fn collapsed_tool_hides_output() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ list_files 0.5s\nfile1\nfile2".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.view.all_collapsed = true;
    let feed = LazyCache::feed(&state);

    let has_tool_done = feed
        .elements
        .iter()
        .any(|e| matches!(e, Element::ToolDone { .. }));
    assert!(!has_tool_done, "Collapsed tool should not render ToolDone");

    let summary = feed.elements.iter().find_map(|e| match e {
        Element::ToolSummary { name, .. } => Some(name.as_str()),
        _ => None,
    });
    assert_eq!(summary, Some("list_files"));
}

#[test]
fn expanded_tool_shows_output() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ list_files 0.5s\nfile1\nfile2".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    let feed = LazyCache::feed(&state);

    let tool_done = feed.elements.iter().find_map(|e| match e {
        Element::ToolDone { output, .. } => Some(output.as_str()),
        _ => None,
    });
    assert!(tool_done.is_some());
    assert!(
        tool_done.unwrap().contains("file1"),
        "Expanded tool should show output"
    );
}
