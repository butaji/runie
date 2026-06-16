use crate::dsl::AppStateDsl;
use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use crate::event::Event;
use crate::model::{AppState, Role};

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn test_agent_thinking_sets_streaming() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent("req.0").think();
    assert!(state.agent.streaming);
    assert!(state.agent.thinking_started_at.is_some());
}

#[test]
fn test_agent_response_creates_message() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent("req.0").think().thought_done().respond("Hello");
    assert_eq!(state.session.messages.len(), 2);
    assert_eq!(state.session.messages[1].role, Role::Assistant);
    assert_eq!(state.session.messages[1].content, "Hello");
}

#[test]
fn test_agent_response_appends_to_existing() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent("req.0").think().thought_done();
    state.agent("req.0").respond("Hello ");
    state.agent("req.0").respond("World");
    assert_eq!(state.session.messages.len(), 2);
    assert_eq!(state.session.messages[0].role, Role::Thought);
    assert_eq!(state.session.messages[1].role, Role::Assistant);
    assert_eq!(state.session.messages[1].content, "Hello World");
}

#[test]
fn test_agent_done_clears_streaming() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent.thinking_started_at = Some(std::time::Instant::now());
    state.agent("req.0").done();
    assert!(!state.agent.streaming);
    assert!(state.agent.thinking_started_at.is_none());
}

#[test]
fn test_agent_error_creates_error_message() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent("req.0").error("Something went wrong");
    assert!(!state.agent.streaming);
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].role, Role::Assistant);
    assert!(state.session.messages[0].content.contains("Error"));
}

#[test]
fn agent_message_strips_tool_markers_on_done() {
    let mut state = fresh_state();
    state.agent("req.0").respond("TOOL:list_dir.").done();
    let has_tool = state
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::Assistant && m.content.contains("TOOL:"));
    assert!(!has_tool);
}

#[test]
fn agent_message_keeps_natural_language() {
    let mut state = fresh_state();
    state
        .agent("req.0")
        .respond("Let me check.\nTOOL:list_dir.")
        .done();
    let msg = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Assistant)
        .unwrap();
    assert_eq!(msg.content, "Let me check.");
}

#[test]
fn agent_message_removes_empty_after_strip() {
    let mut state = fresh_state();
    state.agent("req.0").respond("TOOL:list_dir.").done();
    let count = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .count();
    assert_eq!(count, 0);
}

#[test]
fn agent_message_strips_structured_tool() {
    let mut state = fresh_state();
    state
        .agent("req.0")
        .respond(
            r#"{"name": "edit_file", "arguments": {"path": "x", "search": "a", "replace": "b"}}"#,
        )
        .done();
    let count = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .count();
    assert_eq!(count, 0);
}

#[test]
fn streaming_append_updates_timestamp() {
    let mut state = fresh_state();
    state.agent("req.0").respond("Hello ");
    let t1 = state.session.messages[0].timestamp;
    state.agent("req.0").respond("World");
    let t2 = state.session.messages[0].timestamp;
    assert!(
        t2 >= t1,
        "Timestamp should not go backwards, got t1={} t2={}",
        t1,
        t2
    );
}

#[test]
fn tool_end_updates_timestamp() {
    let mut state = fresh_state();
    state.agent("req.0").tool_start("list_files");
    let t1 = state.session.messages[0].timestamp;
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.5,
        output: String::new(),
    }));
    let t2 = state.session.messages[0].timestamp;
    assert!(
        t2 >= t1,
        "Timestamp should not go backwards, got t1={} t2={}",
        t1,
        t2
    );
}

#[test]
fn thought_marker_comes_before_response_in_event_order() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent("req.0").think().respond("Hello").thought_done();
    let roles: Vec<&str> = state
        .session
        .messages
        .iter()
        .map(|m| m.role.as_str())
        .collect();
    assert_eq!(roles, vec!["thought", "assistant"]);
}

#[test]
fn thought_marker_ordered_by_timestamp_in_feed() {
    use crate::ui::LazyCache;
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent("req.0").think().respond("Hello").thought_done();
    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed
        .elements
        .iter()
        .map(|e| match e {
            crate::ui::Element::ThoughtMarker { .. }
            | crate::ui::Element::ThoughtSummary { .. } => "T",
            crate::ui::Element::AgentMessage { .. } => "A",
            crate::ui::Element::Spacer { .. } => "S",
            _ => "?",
        })
        .collect();
    assert_eq!(kinds, vec!["S", "A", "S", "T", "S"]);
}

#[test]
fn thinking_indicator_ordered_by_timestamp() {
    use crate::ui::LazyCache;
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent("req.0").think().respond("Hello");
    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed
        .elements
        .iter()
        .map(|e| match e {
            crate::ui::Element::Thinking { .. } => "I",
            crate::ui::Element::AgentMessage { .. } => "A",
            crate::ui::Element::Spacer { .. } => "S",
            _ => "?",
        })
        .collect();
    assert_eq!(
        kinds,
        vec!["S", "I", "S"],
        "Only thinking indicator visible during thinking"
    );
}

#[test]
fn streaming_tool_marker_stored_for_thought_capture() {
    let mut state = fresh_state();
    state.agent("req.0").respond("TOOL:list_dir.");
    let msg = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Assistant)
        .unwrap();
    assert!(
        msg.content.contains("TOOL:"),
        "Tool markers stored for thought capture"
    );
    let feed = crate::ui::LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::ui::Element::AgentMessage { content, .. } => content.contains("TOOL:"),
        _ => false,
    });
    assert!(
        !has_tool,
        "TOOL: marker should never appear in rendered feed"
    );
}

#[test]
fn streaming_mixed_text_and_tool_keeps_both_for_capture() {
    let mut state = fresh_state();
    state
        .agent("req.0")
        .respond("Let me check files.\nTOOL:list_dir.");
    let msg = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Assistant)
        .unwrap();
    assert!(msg.content.contains("Let me check files."));
    assert!(
        msg.content.contains("TOOL:list_dir."),
        "Both stored for thought capture"
    );
}

#[test]
fn streaming_structured_tool_stored_for_capture() {
    let mut state = fresh_state();
    state.agent("req.0").respond(
        r#"{"name": "edit_file", "arguments": {"path": "x", "search": "a", "replace": "b"}}"#,
    );
    let msg = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Assistant)
        .unwrap();
    assert!(
        msg.content.contains("edit_file"),
        "Structured tool stored for thought capture"
    );
    let feed = crate::ui::LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::ui::Element::AgentMessage { content, .. } => content.contains("edit_file"),
        _ => false,
    });
    assert!(
        !has_tool,
        "Structured tool call should never appear in rendered feed"
    );
}

#[test]
fn feed_does_not_render_tool_markers() {
    use crate::ui::LazyCache;
    let mut state = fresh_state();
    state.agent("req.0").respond("TOOL:list_dir.");
    let feed = LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::ui::Element::AgentMessage { content, .. } => content.contains("TOOL:"),
        _ => false,
    });
    assert!(!has_tool, "Feed should never render TOOL: markers");
}

#[test]
fn message_stores_provider() {
    let mut state = fresh_state();
    state.config.current_provider = "anthropic".to_string();
    state.agent("req.0").respond("Hello");
    let msg = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Assistant)
        .unwrap();
    assert_eq!(msg.provider, "anthropic");
}
