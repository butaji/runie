use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;
use crate::ui::LazyCache;
use crate::ui::elements::Element;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn thought_created_via_pipeline_is_expanded() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "I'll list files.\n".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:list_dir:.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    let thought = state.messages.iter().find(|m| m.role == Role::Thought).unwrap();
    assert!(!state.collapsed.contains(&thought.id), "Thought should be expanded by default, only hotkey toggles");
}

#[test]
fn tool_created_via_pipeline_is_expanded() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_dir".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1\nfile2".to_string() });

    let tool = state.messages.iter().find(|m| m.role == Role::Tool).unwrap();
    assert!(!state.collapsed.contains(&tool.id), "Tool should be expanded by default, only hotkey toggles");
}

#[test]
fn toggle_expand_collapses_expanded_thought() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "I'll list files.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    let thought_id = state.messages.iter().find(|m| m.role == Role::Thought).unwrap().id.clone();
    assert!(!state.collapsed.contains(&thought_id), "Should start expanded");

    state.update(Event::ToggleExpand);
    assert!(state.collapsed.contains(&thought_id), "ToggleExpand should collapse the thought");
}

#[test]
fn toggle_expand_collapses_expanded_tool() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_dir".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1".to_string() });

    let tool_id = state.messages.iter().find(|m| m.role == Role::Tool).unwrap().id.clone();
    assert!(!state.collapsed.contains(&tool_id), "Should start expanded");

    state.update(Event::ToggleExpand);
    assert!(state.collapsed.contains(&tool_id), "ToggleExpand should collapse the tool");
}

#[test]
fn thought_expanded_by_default() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Thinking...".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    let feed = LazyCache::feed(&state);
    let has_full = feed.elements.iter().any(|e| matches!(e, Element::ThoughtMarker { .. }));
    assert!(has_full, "Thought should render by default");
}

#[test]
fn toggle_expand_hides_thought() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning here\nline two".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleExpand);
    assert!(state.collapsed.contains("t1"), "Thought id should be in collapsed set");
}

#[test]
fn toggle_expand_restores_thought() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleExpand);
    state.update(Event::ToggleExpand);
    assert!(!state.collapsed.contains("t1"), "Thought id should be removed from collapsed set");
}

#[test]
fn collapsed_thought_renders_one_line_summary() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning\nline two\nline three".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.collapsed.insert("t1".into());
    let feed = LazyCache::feed(&state);
    let summary = feed.elements.iter().find_map(|e| match e {
        Element::ThoughtSummary { content, .. } => Some(content.as_str()),
        _ => None,
    });
    assert!(summary.is_some(), "Collapsed thought should render as ThoughtSummary");
    assert!(summary.unwrap().contains("Deep reasoning"), "Summary should contain first line");
}

#[test]
fn tool_collapsed_by_toggle() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleExpand);
    assert!(state.collapsed.contains("t1"), "Tool id should be in collapsed set");
}

#[test]
fn toggle_expand_restores_tool() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleExpand);
    state.update(Event::ToggleExpand);
    assert!(!state.collapsed.contains("t1"), "Tool id should be removed from collapsed set");
}

#[test]
fn collapsed_tool_renders_one_line_summary() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.collapsed.insert("t1".into());
    let feed = LazyCache::feed(&state);
    let summary = feed.elements.iter().find_map(|e| match e {
        Element::ToolSummary { name, .. } => Some(name.as_str()),
        _ => None,
    });
    assert!(summary.is_some(), "Collapsed tool should render as ToolSummary");
    assert_eq!(summary.unwrap(), "list_files", "Summary should show tool name");
}

#[test]
fn toggle_expand_noop_when_empty() {
    let mut state = fresh_state();
    state.update(Event::ToggleExpand);
    assert!(state.collapsed.is_empty());
}

#[test]
fn toggle_expand_prefers_most_recent() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "older thought".into(),
        timestamp: 0.0,
        id: "old".into(),
    });
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s".into(),
        timestamp: 1.0,
        id: "new".into(),
    });
    state.update(Event::ToggleExpand);
    assert!(!state.collapsed.contains("old"), "Should not toggle older thought");
    assert!(state.collapsed.contains("new"), "Should toggle most recent tool");
}

#[test]
fn toggle_thought_rebuilds_cache() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning\nline two".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.ensure_fresh();
    let before = state.elements_cache().to_vec();
    assert!(before.iter().any(|e| matches!(e, Element::ThoughtMarker { .. })));

    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    let after = state.elements_cache().to_vec();
    assert!(
        after.iter().any(|e| matches!(e, Element::ThoughtSummary { .. })),
        "Cache should rebuild to ThoughtSummary after toggle"
    );
}

#[test]
fn toggle_thought_twice_restores_cache() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    let cache = state.elements_cache().to_vec();
    assert!(
        cache.iter().any(|e| matches!(e, Element::ThoughtMarker { .. })),
        "Cache should restore ThoughtMarker after second toggle"
    );
}

#[test]
fn toggle_tool_rebuilds_cache() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.ensure_fresh();
    let before = state.elements_cache().to_vec();
    assert!(before.iter().any(|e| matches!(e, Element::ToolDone { .. })));

    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    let after = state.elements_cache().to_vec();
    assert!(
        after.iter().any(|e| matches!(e, Element::ToolSummary { .. })),
        "Cache should rebuild to ToolSummary after toggle"
    );
}

#[test]
fn toggle_tool_twice_restores_cache() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    let cache = state.elements_cache().to_vec();
    assert!(
        cache.iter().any(|e| matches!(e, Element::ToolDone { .. })),
        "Cache should restore ToolDone after second toggle"
    );
}

#[test]
fn thought_captures_assistant_reasoning() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "I'll list the files.\n".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:list_dir:.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    let thought = state.messages.iter().find(|m| m.role == Role::Thought).unwrap();
    assert!(thought.content.contains("I'll list the files."), "Thought should capture reasoning: {}", thought.content);
    assert!(!thought.content.contains("TOOL:"), "Thought should have tool markers stripped");
}

#[test]
fn assistant_preserved_when_no_tools() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Here is the answer.".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });

    let assistants: Vec<_> = state.messages.iter().filter(|m| m.role == Role::Assistant).collect();
    assert_eq!(assistants.len(), 1, "Assistant should be preserved when no tools");
    assert_eq!(assistants[0].content, "Here is the answer.");
}

#[test]
fn tool_stores_output() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_dir".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1\nfile2".to_string() });

    let tool = state.messages.iter().find(|m| m.role == Role::Tool).unwrap();
    assert!(tool.content.contains("file1"), "Tool should store output: {}", tool.content);
    assert!(tool.content.contains("file2"), "Tool should store output: {}", tool.content);
}

#[test]
fn collapsed_thought_hides_reasoning() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.2s\nI'll list the files.".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.collapsed.insert("t1".into());
    let feed = LazyCache::feed(&state);

    let summary = feed.elements.iter().find_map(|e| match e {
        Element::ThoughtSummary { content, .. } => Some(content.as_str()),
        _ => None,
    });
    assert!(summary.is_some());
    assert!(!summary.unwrap().contains("I'll list"), "Collapsed thought should hide reasoning");
}

#[test]
fn expanded_thought_shows_reasoning() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.2s\nI'll list the files.".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    let feed = LazyCache::feed(&state);

    let marker = feed.elements.iter().find_map(|e| match e {
        Element::ThoughtMarker { content } => Some(content.as_str()),
        _ => None,
    });
    assert!(marker.is_some());
    assert!(marker.unwrap().contains("I'll list"), "Expanded thought should show reasoning");
}

#[test]
fn collapsed_tool_hides_output() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s\nfile1\nfile2".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.collapsed.insert("t1".into());
    let feed = LazyCache::feed(&state);

    let has_tool_done = feed.elements.iter().any(|e| matches!(e, Element::ToolDone { .. }));
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
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s\nfile1\nfile2".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    let feed = LazyCache::feed(&state);

    let tool_done = feed.elements.iter().find_map(|e| match e {
        Element::ToolDone { output, .. } => Some(output.as_str()),
        _ => None,
    });
    assert!(tool_done.is_some());
    assert!(tool_done.unwrap().contains("file1"), "Expanded tool should show output");
}
