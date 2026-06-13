//! toggle tests.

use crate::event::Event;
use crate::model::{AppState, ChatMessage, Role};
use crate::ui::elements::Element;
use crate::ui::LazyCache;
fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn thought_created_via_pipeline_is_expanded() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "I'll list files.\n".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "TOOL:list_dir:.".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });
    assert!(
        !state.view.all_collapsed,
        "Thoughts should be expanded by default"
    );
}

#[test]
fn tool_created_via_pipeline_is_expanded() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "list_dir".to_string(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output: "file1\nfile2".to_string(),
    });
    assert!(
        !state.view.all_collapsed,
        "Tools should be expanded by default"
    );
}

#[test]
fn toggle_expand_collapses_all_thoughts() {
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

    assert!(!state.view.all_collapsed, "Should start expanded");
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "ToggleExpand should collapse all thoughts/tools"
    );
}

#[test]
fn toggle_expand_collapses_all_tools() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "list_dir".to_string(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output: "file1".to_string(),
    });

    assert!(!state.view.all_collapsed, "Should start expanded");
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "ToggleExpand should collapse all thoughts/tools"
    );
}

#[test]
fn thought_expanded_by_default() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Thinking...".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    let feed = LazyCache::feed(&state);
    let has_full = feed
        .elements
        .iter()
        .any(|e| matches!(e, Element::ThoughtMarker { .. }));
    assert!(has_full, "Thought should render by default");
}

#[test]
fn toggle_expand_hides_thought() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning here\nline two".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed, "Toggle should set all_collapsed");
}

#[test]
fn toggle_expand_restores_thought() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(Event::ToggleExpand);
    state.update(Event::ToggleExpand);
    assert!(!state.view.all_collapsed, "Second toggle should expand all");
}

#[test]
fn collapsed_thought_renders_one_line_summary() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning\nline two\nline three".into(),
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
    assert!(
        summary.is_some(),
        "Collapsed thought should render as ThoughtSummary"
    );
    assert!(
        summary.unwrap().contains("Deep reasoning"),
        "Summary should contain first line"
    );
}

#[test]
fn tool_collapsed_by_toggle() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed, "Toggle should set all_collapsed");
}

#[test]
fn toggle_expand_restores_tool() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(Event::ToggleExpand);
    state.update(Event::ToggleExpand);
    assert!(!state.view.all_collapsed, "Second toggle should expand all");
}

#[test]
fn collapsed_tool_renders_one_line_summary() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.view.all_collapsed = true;
    let feed = LazyCache::feed(&state);
    let summary = feed.elements.iter().find_map(|e| match e {
        Element::ToolSummary { name, .. } => Some(name.as_str()),
        _ => None,
    });
    assert!(
        summary.is_some(),
        "Collapsed tool should render as ToolSummary"
    );
    assert_eq!(
        summary.unwrap(),
        "list_files",
        "Summary should show tool name"
    );
}

#[test]
fn toggle_expand_noop_when_empty() {
    let mut state = fresh_state();
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "Toggle on empty state should still flip flag"
    );
}
