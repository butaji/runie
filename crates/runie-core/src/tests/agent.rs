#![allow(clippy::all)]
use crate::dsl::AppStateDsl;
use crate::model::Role;
use crate::tests::fresh_state;

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
    assert_eq!(state.session.messages[1].content(), "Hello");
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
    assert_eq!(state.session.messages[1].content(), "Hello World");
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
    assert!(state.session.messages[0].content().contains("Error"));
}

#[test]
fn agent_message_strips_tool_markers_on_done() {
    let mut state = fresh_state();
    state.agent("req.0").respond("TOOL:list_dir.").done();
    let has_tool = state
        .session
        .messages
        .iter()
        .any(|m| m.role == Role::Assistant && m.content().contains("TOOL:"));
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
    assert_eq!(msg.content(), "Let me check.");
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
fn agent_message_strips_tool_call_markup() {
    let mut state = fresh_state();
    state
        .agent("req.0")
        .respond(r#"[TOOL_CALL]{tool => "bash", args => {"command" => "ls"}}[/TOOL_CALL]"#)
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
fn agent_message_keeps_natural_language_around_tool_call_markup() {
    let mut state = fresh_state();
    state
        .agent("req.0")
        .respond("I will list files.\n[TOOL_CALL]{tool => \"list_dir\", args => {\"path\" => \".\"}}[/TOOL_CALL]\nDone.")
        .done();
    let msg = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Assistant)
        .unwrap();
    assert_eq!(msg.content(), "I will list files.\nDone.");
}

#[test]
fn feed_does_not_render_tool_call_markup() {
    use crate::view::LazyCache;
    let mut state = fresh_state();
    state
        .agent("req.0")
        .respond(r#"[TOOL_CALL]{tool => "bash", args => {"command" => "ls"}}[/TOOL_CALL]"#)
        .done();
    let feed = LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::view::Element::AgentMessage { content, .. } => content.contains("[TOOL_CALL]"),
        _ => false,
    });
    assert!(!has_tool, "Feed should never render [TOOL_CALL] markers");
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
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: String::new(),
    
        input: None,});
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
    use crate::view::LazyCache;
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent("req.0").think().respond("Hello").thought_done();
    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed
        .elements
        .iter()
        .map(|e| match e {
            crate::view::Element::ThoughtMarker { .. }
            | crate::view::Element::ThoughtSummary { .. } => "T",
            crate::view::Element::AgentMessage { .. } => "A",
            crate::view::Element::Spacer { .. } => "S",
            _ => "?",
        })
        .collect();
    assert_eq!(kinds, vec!["S", "A", "S", "T", "S"]);
}

#[test]
fn thinking_indicator_ordered_by_timestamp() {
    use crate::view::LazyCache;
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.agent("req.0").think().respond("Hello");
    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed
        .elements
        .iter()
        .map(|e| match e {
            crate::view::Element::Thinking { .. } => "I",
            crate::view::Element::AgentMessage { .. } => "A",
            crate::view::Element::Spacer { .. } => "S",
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
        msg.content().contains("TOOL:"),
        "Tool markers stored for thought capture"
    );
    let feed = crate::view::LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::view::Element::AgentMessage { content, .. } => content.contains("TOOL:"),
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
    assert!(msg.content().contains("Let me check files."));
    assert!(
        msg.content().contains("TOOL:list_dir."),
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
        msg.content().contains("edit_file"),
        "Structured tool stored for thought capture"
    );
    let feed = crate::view::LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::view::Element::AgentMessage { content, .. } => content.contains("edit_file"),
        _ => false,
    });
    assert!(
        !has_tool,
        "Structured tool call should never appear in rendered feed"
    );
}

#[test]
fn feed_does_not_render_tool_markers() {
    use crate::view::LazyCache;
    let mut state = fresh_state();
    state.agent("req.0").respond("TOOL:list_dir.");
    let feed = LazyCache::feed(&state);
    let has_tool = feed.elements.iter().any(|e| match e {
        crate::view::Element::AgentMessage { content, .. } => content.contains("TOOL:"),
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

#[test]
fn assistant_message_preserves_unicode_after_tool_strip() {
    let mut state = fresh_state();
    state.agent("req.0").respond("hello 😊 world").done();
    let msg = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Assistant)
        .expect("assistant message");
    assert_eq!(msg.content(), "hello 😊 world");
}

/// A full tool-turn cycle should render the tool result and the final
/// assistant response, not just a thought marker.
#[test]
fn tool_turn_renders_tool_result_and_final_response() {
    use crate::view::LazyCache;
    let mut state = fresh_state();
    state
        .agent("req.0")
        .think()
        .respond("I'll list files.\nTOOL:list_dir:.")
        .thought_done()
        .tool("list_dir", "Cargo.toml\nsrc/")
        .respond("Done.")
        .done();

    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed
        .elements
        .iter()
        .map(|e| match e {
            crate::view::Element::ToolDone { .. } => "D",
            crate::view::Element::AgentMessage { .. } => "A",
            crate::view::Element::ThoughtMarker { .. } => "T",
            _ => "?",
        })
        .collect();

    assert!(
        kinds.iter().any(|k| *k == "D"),
        "tool result should render in feed, got kinds {:?}",
        kinds
    );
    assert!(
        kinds.iter().any(|k| *k == "A"),
        "final assistant response should render in feed, got kinds {:?}",
        kinds
    );
}
