//! toggle tests.

use runie_core::event::{AgentEvent, ControlEvent};
use runie_core::model::{AppState, ChatMessage,  Role};
use runie_core::Part;
use runie_core::ui::elements::Element;
use runie_core::ui::LazyCache;
use runie_testing::fresh_state;

#[test]
fn thought_created_via_pipeline_is_expanded() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "I'll list files.\n".to_string(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "TOOL:list_dir:.".to_string(),
    });
    state.update(AgentEvent::ThoughtDone {
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
    assert!(
        !state.view.all_collapsed,
        "Tools should be expanded by default"
    );
}

#[test]
fn toggle_expand_collapses_all_thoughts() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "I'll list files.".to_string(),
    });
    state.update(AgentEvent::ThoughtDone {
        id: "req.0".to_string(),
    });

    assert!(!state.view.all_collapsed, "Should start expanded");
    state.update(ControlEvent::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "ToggleExpand should collapse all thoughts/tools"
    );
}

#[test]
fn toggle_expand_collapses_all_tools() {
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
        output: "file1".to_string(),
    });

    assert!(!state.view.all_collapsed, "Should start expanded");
    state.update(ControlEvent::ToggleExpand);
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
        parts: vec![Part::Text { content: "Thinking...".into() }],
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
        parts: vec![Part::Text { content: "Deep reasoning here\nline two".into() }],
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(ControlEvent::ToggleExpand);
    assert!(state.view.all_collapsed, "Toggle should set all_collapsed");
}

#[test]
fn toggle_expand_restores_thought() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: "Deep reasoning".into() }],
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(ControlEvent::ToggleExpand);
    state.update(ControlEvent::ToggleExpand);
    assert!(!state.view.all_collapsed, "Second toggle should expand all");
}

#[test]
fn collapsed_thought_renders_one_line_summary() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: "Deep reasoning\nline two\nline three".into() }],
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
        parts: vec![Part::Text { content: "✓ list_files 0.5s".into() }],
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(ControlEvent::ToggleExpand);
    assert!(state.view.all_collapsed, "Toggle should set all_collapsed");
}

#[test]
fn toggle_expand_restores_tool() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text { content: "✓ list_files 0.5s".into() }],
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(ControlEvent::ToggleExpand);
    state.update(ControlEvent::ToggleExpand);
    assert!(!state.view.all_collapsed, "Second toggle should expand all");
}

#[test]
fn collapsed_tool_renders_one_line_summary() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text { content: "✓ list_files 0.5s".into() }],
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
    state.update(ControlEvent::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "Toggle on empty state should still flip flag"
    );
}
