use crate::model::{AppState, Role};
use crate::event::Event;
use crate::ui::LazyCache;

fn fresh_state() -> AppState {
    AppState::default()
}

fn has_agent_message(state: &AppState) -> bool {
    let feed = LazyCache::feed(state);
    feed.elements.iter().any(|e| matches!(e, crate::ui::Element::AgentMessage { .. }))
}

fn agent_texts(state: &AppState) -> Vec<String> {
    let feed = LazyCache::feed(state);
    feed.elements.iter().filter_map(|e| match e {
        crate::ui::Element::AgentMessage { content } => Some(content.clone()),
        _ => None,
    }).collect()
}

// ── Core rule: assistant with tool markers must NOT render ────────────

#[test]
fn assistant_with_tool_marker_not_rendered() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".into(), content: "TOOL:list_dir.".into() });
    state.ensure_fresh();

    assert!(!has_agent_message(&state),
        "Assistant containing only tool marker must not render as AgentMessage");
}

#[test]
fn assistant_with_mixed_text_and_tool_not_rendered() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\nTOOL:list_dir:.".into() });
    state.ensure_fresh();

    assert!(!has_agent_message(&state),
        "Assistant with natural language + tool marker must not render as AgentMessage (captured in thought)");
}

#[test]
fn assistant_with_structured_tool_not_rendered() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".into(), content: r#"{"name": "edit_file", "arguments": {"path": "x", "search": "a", "replace": "b"}}"#.into() });
    state.ensure_fresh();

    assert!(!has_agent_message(&state),
        "Assistant with structured tool call must not render as AgentMessage");
}

// ── Natural language assistant SHOULD render ──────────────────────────

#[test]
fn assistant_pure_text_renders_normally() {
    let mut state = fresh_state();
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello world".into() });
    state.ensure_fresh();

    assert!(has_agent_message(&state),
        "Pure text assistant response must render as AgentMessage");
    assert_eq!(agent_texts(&state), vec!["Hello world"]);
}

// ── During thinking phase: NO assistant renders ───────────────────────

#[test]
fn no_agent_during_thinking_phase() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Let me think...".into() });
    state.ensure_fresh();

    assert!(!has_agent_message(&state),
        "Assistant must not render during thinking phase (will be captured in thought)");
}

#[test]
fn no_agent_during_thinking_even_with_tool() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\nTOOL:list_dir:.".into() });
    state.ensure_fresh();

    assert!(!has_agent_message(&state),
        "Assistant must not render during thinking phase");
}

// ── After thought_done: thought renders, assistant removed ────────────

#[test]
fn thought_renders_after_thought_done() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\nTOOL:list_dir:.".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.ensure_fresh();

    let feed = LazyCache::feed(&state);
    let has_thought = feed.elements.iter().any(|e| matches!(e, crate::ui::Element::ThoughtMarker { .. }));
    assert!(has_thought, "Thought must render after AgentThoughtDone");
    assert!(!has_agent_message(&state),
        "Ghost AgentMessage must not appear after thought captures it");
}

// ── After tool: post-tool assistant response renders ──────────────────

#[test]
fn post_tool_assistant_renders() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\nTOOL:list_dir:.".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done!".into() });
    state.ensure_fresh();

    assert!(has_agent_message(&state),
        "Post-tool assistant response must render");
    assert_eq!(agent_texts(&state), vec!["Done!"]);
}

// ── Full turn: no ghost "Agent:" during preparation ──────────────────

#[test]
fn full_turn_no_ghost_agent_messages() {
    let mut state = fresh_state();
    state.streaming = true;

    // User submits
    state.input = "list files".into();
    state.update(Event::Submit);
    state.ensure_fresh();
    assert!(agent_texts(&state).is_empty(), "No agent msg before agent responds");

    // Agent thinks
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.ensure_fresh();
    assert!(agent_texts(&state).is_empty(), "No agent msg during thinking");

    // Agent sends response with tool
    state.update(Event::AgentResponse { id: "req.0".into(), content: "I'll list the files.\nTOOL:list_dir:.".into() });
    state.ensure_fresh();
    assert!(agent_texts(&state).is_empty(),
        "Must NOT see 'Agent: I'll list the files' — it will be captured in thought");

    // Thought done
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.ensure_fresh();
    assert!(agent_texts(&state).is_empty(),
        "No ghost agent after thought_done captures the text");

    // Tool runs
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a\nb\nc".into() });
    state.ensure_fresh();
    assert!(agent_texts(&state).is_empty(), "No agent msg during tool execution");

    // Final response
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Here are your files.".into() });
    state.ensure_fresh();
    assert_eq!(agent_texts(&state), vec!["Here are your files."],
        "Final response must render as AgentMessage");

    // Turn complete
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 2.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();
    assert_eq!(agent_texts(&state), vec!["Here are your files."],
        "Final response must persist after turn completes");
}

// ── Streaming chunks: no flicker ─────────────────────────────────────

#[test]
fn streaming_chunks_no_flicker() {
    let mut state = fresh_state();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });

    // Chunk 1: natural language only
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Let me ".into() });
    state.ensure_fresh();
    assert!(!has_agent_message(&state), "No agent during thinking — chunk 1");

    // Chunk 2: more natural language
    state.update(Event::AgentResponse { id: "req.0".into(), content: "check ".into() });
    state.ensure_fresh();
    assert!(!has_agent_message(&state), "No agent during thinking — chunk 2");

    // Chunk 3: tool marker arrives
    state.update(Event::AgentResponse { id: "req.0".into(), content: "TOOL:list_dir:.".into() });
    state.ensure_fresh();
    assert!(!has_agent_message(&state), "No agent after tool marker — chunk 3");
}

// ── Error messages still render ───────────────────────────────────────

#[test]
fn error_message_renders_normally() {
    let mut state = fresh_state();
    state.update(Event::AgentError { id: "req.0".into(), message: "API timeout".into() });
    state.ensure_fresh();

    assert!(has_agent_message(&state), "Error message must render as AgentMessage");
    let texts = agent_texts(&state);
    assert!(texts[0].contains("API timeout"), "Error text must be visible");
}
