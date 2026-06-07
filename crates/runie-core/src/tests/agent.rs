use crate::model::{AppState, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_agent_thinking_sets_streaming() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    assert!(state.streaming);
    assert!(state.thinking_started_at.is_some());
}

#[test]
fn test_agent_response_creates_message() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[1].role, Role::Assistant);
    assert_eq!(state.messages[1].content, "Hello");
}

#[test]
fn test_agent_response_appends_to_existing() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello ".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "World".to_string() });
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].role, Role::Thought);
    assert_eq!(state.messages[1].role, Role::Assistant);
    assert_eq!(state.messages[1].content, "Hello World");
}

#[test]
fn test_agent_done_clears_streaming() {
    let mut state = fresh_state();
    state.streaming = true;
    state.thinking_started_at = Some(std::time::Instant::now());
    state.update(Event::AgentDone { id: "req.0".to_string() });
    assert!(!state.streaming);
    assert!(state.thinking_started_at.is_none());
}

#[test]
fn test_agent_error_creates_error_message() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentError { id: "req.0".to_string(), message: "Something went wrong".to_string() });
    assert!(!state.streaming);
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, Role::Assistant);
    assert!(state.messages[0].content.contains("Error"));
}

#[test]
fn agent_message_strips_tool_markers_on_done() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:list_dir.".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    let has_tool = state.messages.iter().any(|m| m.role == Role::Assistant && m.content.contains("TOOL:"));
    assert!(!has_tool);
}

#[test]
fn agent_message_keeps_natural_language() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Let me check.\nTOOL:list_dir.".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    let msg = state.messages.iter().find(|m| m.role == Role::Assistant).unwrap();
    assert_eq!(msg.content, "Let me check.");
}

#[test]
fn agent_message_removes_empty_after_strip() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:list_dir.".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    let count = state.messages.iter().filter(|m| m.role == Role::Assistant).count();
    assert_eq!(count, 0);
}

#[test]
fn agent_message_strips_structured_tool() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: r#"{"name": "edit_file", "arguments": {"path": "x", "search": "a", "replace": "b"}}"#.to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    let count = state.messages.iter().filter(|m| m.role == Role::Assistant).count();
    assert_eq!(count, 0);
}

#[test]
fn streaming_append_updates_timestamp() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello ".to_string() });
    let t1 = state.messages[0].timestamp;
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "World".to_string() });
    let t2 = state.messages[0].timestamp;
    assert!(t2 > t1, "Timestamp should update on streaming merge, got t1={} t2={}", t1, t2);
}

#[test]
fn tool_end_updates_timestamp() {
    let mut state = fresh_state();
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
    let t1 = state.messages[0].timestamp;
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: String::new() });
    let t2 = state.messages[0].timestamp;
    assert!(t2 > t1, "Timestamp should update on tool end, got t1={} t2={}", t1, t2);
}

#[test]
fn thought_marker_comes_before_response_in_event_order() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    let roles: Vec<&str> = state.messages.iter().map(|m| m.role.as_str()).collect();
    assert_eq!(roles, vec!["thought", "assistant"], "Thought marker must come before response even when response chunks arrive earlier");
}

#[test]
fn thought_marker_comes_before_response_in_feed() {
    use crate::ui::LazyCache;
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
        crate::ui::Element::ThoughtMarker { .. } => "T",
        crate::ui::Element::AgentMessage { .. } => "A",
        crate::ui::Element::Spacer => "S",
        _ => "?",
    }).collect();
    assert_eq!(kinds, vec!["T", "S", "A", "S"], "Feed must render thought before agent response");
}

#[test]
fn thinking_indicator_comes_before_response() {
    use crate::ui::LazyCache;
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
        crate::ui::Element::Thinking { .. } => "I",
        crate::ui::Element::AgentMessage { .. } => "A",
        crate::ui::Element::Spacer => "S",
        _ => "?",
    }).collect();
    assert_eq!(kinds, vec!["I", "S", "A", "S"], "Thinking indicator must render before agent response");
}

#[test]
fn streaming_tool_marker_stored_for_thought_capture() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:list_dir.".to_string() });
    let msg = state.messages.iter().find(|m| m.role == Role::Assistant).unwrap();
    assert!(msg.content.contains("TOOL:"), "Tool markers stored for thought capture, stripped at render time");
    let feed = crate::ui::LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::ui::Element::AgentMessage { content } => content.contains("TOOL:"),
        _ => false,
    });
    assert!(!has_tool, "TOOL: marker should never appear in rendered feed");
}

#[test]
fn streaming_mixed_text_and_tool_keeps_both_for_capture() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Let me check files.\nTOOL:list_dir.".to_string() });
    let msg = state.messages.iter().find(|m| m.role == Role::Assistant).unwrap();
    assert!(msg.content.contains("Let me check files."));
    assert!(msg.content.contains("TOOL:list_dir."), "Both stored for thought capture");
}

#[test]
fn streaming_structured_tool_stored_for_capture() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: r#"{"name": "edit_file", "arguments": {"path": "x", "search": "a", "replace": "b"}}"#.to_string() });
    let msg = state.messages.iter().find(|m| m.role == Role::Assistant).unwrap();
    assert!(msg.content.contains("edit_file"), "Structured tool stored for thought capture, stripped at render time");
    let feed = crate::ui::LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::ui::Element::AgentMessage { content } => content.contains("edit_file"),
        _ => false,
    });
    assert!(!has_tool, "Structured tool call should never appear in rendered feed");
}

#[test]
fn feed_does_not_render_tool_markers() {
    use crate::ui::LazyCache;
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "TOOL:list_dir.".to_string() });
    let feed = LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::ui::Element::AgentMessage { content } => content.contains("TOOL:"),
        _ => false,
    });
    assert!(!has_tool, "Feed should never render TOOL: markers");
}
