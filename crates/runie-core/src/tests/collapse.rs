use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;
use crate::ui::LazyCache;
use crate::ui::elements::Element;

fn fresh_state() -> AppState {
    AppState::default()
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
fn toggle_thought_hides_content() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning here\nline two".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleThought);
    assert!(state.collapsed_thoughts.contains("t1"), "Thought id should be in collapsed set");
}

#[test]
fn toggle_thought_restores_content() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleThought);
    state.update(Event::ToggleThought);
    assert!(!state.collapsed_thoughts.contains("t1"), "Thought id should be removed from collapsed set");
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
    state.collapsed_thoughts.insert("t1".into());
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
    state.update(Event::ToggleTool);
    assert!(state.collapsed_tools.contains("t1"), "Tool id should be in collapsed set");
}

#[test]
fn toggle_tool_restores_content() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleTool);
    state.update(Event::ToggleTool);
    assert!(!state.collapsed_tools.contains("t1"), "Tool id should be removed from collapsed set");
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
    state.collapsed_tools.insert("t1".into());
    let feed = LazyCache::feed(&state);
    let summary = feed.elements.iter().find_map(|e| match e {
        Element::ToolSummary { name, .. } => Some(name.as_str()),
        _ => None,
    });
    assert!(summary.is_some(), "Collapsed tool should render as ToolSummary");
    assert_eq!(summary.unwrap(), "list_files", "Summary should show tool name");
}

#[test]
fn toggle_thought_noop_when_no_thoughts() {
    let mut state = fresh_state();
    state.update(Event::ToggleThought);
    assert!(state.collapsed_thoughts.is_empty());
}

#[test]
fn toggle_tool_noop_when_no_tools() {
    let mut state = fresh_state();
    state.update(Event::ToggleTool);
    assert!(state.collapsed_tools.is_empty());
}

#[test]
fn toggle_collapse_by_index_works() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleCollapse { index: 0 });
    assert!(state.collapsed_thoughts.contains("t1"));
    state.update(Event::ToggleCollapse { index: 0 });
    assert!(!state.collapsed_thoughts.contains("t1"));
}

#[test]
fn toggle_collapse_out_of_range_is_noop() {
    let mut state = fresh_state();
    state.update(Event::ToggleCollapse { index: 999 });
    assert!(state.collapsed_thoughts.is_empty());
    assert!(state.collapsed_tools.is_empty());
}
