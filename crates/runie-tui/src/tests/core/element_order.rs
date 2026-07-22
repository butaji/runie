use runie_core::model::{AppState, ChatMessage, Role};
use runie_core::view::LazyCache;
use runie_core::Event;
use runie_core::Part;

fn element_kinds(state: &AppState) -> Vec<String> {
    let feed = LazyCache::feed(state);
    feed.elements
        .iter()
        .map(|e| match e {
            runie_core::view::Element::UserMessage { .. } => "User".to_string(),
            runie_core::view::Element::AgentMessage { .. } => "Agent".to_string(),
            runie_core::view::Element::Thinking { .. } => "Thinking".to_string(),
            // A thought post renders as a full marker or (by default) a
            // one-line summary — both are the same post for ordering.
            runie_core::view::Element::ThoughtMarker { .. } | runie_core::view::Element::ThoughtSummary { .. } => {
                "Thought".to_string()
            }
            runie_core::view::Element::AnthropicThinking { .. } => "Thinking".to_string(),
            runie_core::view::Element::ToolRunning { .. } => "ToolRun".to_string(),
            runie_core::view::Element::ToolDone { .. } => "ToolDone".to_string(),
            runie_core::view::Element::ToolSummary { .. } => "ToolSum".to_string(),
            runie_core::view::Element::ToolConfirmation { .. } => "Confirm".to_string(),
            runie_core::view::Element::ContextGroup { .. } => "Context".to_string(),
            runie_core::view::Element::SubagentRow { .. } => "Subagent".to_string(),
            runie_core::view::Element::TurnComplete { .. } => "Turn".to_string(),
            runie_core::view::Element::Spacer { .. } => "Spacer".to_string(),
            runie_core::view::Element::Image { .. } => "Image".to_string(),
            runie_core::view::Element::DataPart { .. } => "Data".to_string(),
            runie_core::view::Element::MarkdownTable { .. } => "Table".to_string(),
            runie_core::view::Element::DiffOutput { .. } => "Diff".to_string(),
            runie_core::view::Element::WebSearchCall { .. } => "Search".to_string(),
            runie_core::view::Element::AnsiStyled { .. } => "ANSI".to_string(),
        })
        .collect()
}

fn element_kinds_no_spacer(state: &AppState) -> Vec<String> {
    element_kinds(state)
        .into_iter()
        .filter(|k| k != "Spacer")
        .collect()
}

fn msg(role: Role, content: &str, timestamp: f64, id: &str) -> ChatMessage {
    ChatMessage {
        role,
        parts: vec![Part::Text { content: content.into() }],
        timestamp,
        id: id.into(),
        ..Default::default()
    }
}

#[test]
fn elements_ordered_by_timestamp_strict() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text { content: "✓ ls 0.5s\noutput".into() }],
        timestamp: 2.0,
        id: "tool.req.0.1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        parts: vec![Part::Text { content: "world".into() }],
        timestamp: 3.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: "◆ Thought 1.0s\nhello".into() }],
        timestamp: 4.0,
        id: "req.0#thought.0".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds,
        vec!["ToolDone", "Agent", "Thought"],
        "Elements must be strictly ordered by timestamp: Tool(2.0) < Agent(3.0) < Thought(4.0)"
    );
}

#[test]
fn newer_assistant_appears_after_older_thought() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: "◆ Thought 1.0s".into() }],
        timestamp: 1.0,
        id: "req.0#thought.0".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        parts: vec![Part::Text { content: "updated later".into() }],
        timestamp: 5.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds,
        vec!["Thought", "Agent"],
        "Thought(1.0) must appear before Agent(5.0) by timestamp"
    );
}

#[test]
fn thinking_indicator_is_always_last_when_newest() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "hello".into() }],
        timestamp: 1.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        parts: vec![Part::Text { content: "hi".into() }],
        timestamp: 2.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.agent.thinking_started_at = Some(std::time::Instant::now());
    state.refresh_after_message_change();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds,
        vec!["User", "Agent", "Thinking"],
        "Thinking indicator (max_ts+1) must be last"
    );
}

#[test]
fn streaming_bump_moves_assistant_to_end() {
    let mut state = AppState::default();
    state.session.messages.extend([
        msg(Role::User, "Q1", 1.0, "u0"),
        msg(Role::Assistant, "A1", 2.0, "req.0"),
        msg(Role::User, "Q2", 3.0, "u1"),
    ]);
    state
        .session
        .messages
        .iter_mut()
        .find(|m| m.role == Role::Assistant && m.id == "req.0")
        .unwrap()
        .timestamp = 4.0;
    state.refresh_after_message_change();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds,
        vec!["User", "User", "Agent"],
        "Agent bumped to 4.0 must appear after User at 3.0"
    );
}

#[test]
fn tool_end_bump_moves_tool_after_later_messages() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text { content: "Running ls...".into() }],
        timestamp: 2.0,
        id: "tool.req.0.1".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "next".into() }],
        timestamp: 3.0,
        id: "u1".into(),
        ..Default::default()
    });
    // Tool completes — bump to 5.0
    if let Some(msg) = state
        .session
        .messages
        .iter_mut()
        .find(|m| m.role == Role::Tool)
    {
        msg.timestamp = 5.0;
        msg.set_text_part("✓ ls 0.5s\noutput".into());
    }
    state.refresh_after_message_change();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds,
        vec!["User", "ToolDone"],
        "Tool bumped to 5.0 must appear after User at 3.0"
    );
}

#[test]
fn multiple_tools_ordered_by_completion_time() {
    let mut state = AppState::default();
    state.session.messages.extend([
        msg(Role::Tool, "✓ cat 0.1s", 5.0, "t1"),
        msg(Role::Tool, "✓ ls 0.2s", 2.0, "t2"),
        msg(Role::Tool, "✓ grep 0.3s", 8.0, "t3"),
    ]);
    state.refresh_after_message_change();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds,
        vec!["ToolDone", "ToolDone", "ToolDone"],
        "Tools should be ordered by timestamp"
    );
    let feed = LazyCache::feed(&state);
    let texts: Vec<String> = feed
        .elements
        .iter()
        .filter_map(|e| match e {
            runie_core::view::Element::ToolDone { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        vec!["ls", "cat", "grep"],
        "Tool order must follow timestamp: ls(2) < cat(5) < grep(8)"
    );
}

#[test]
fn thought_before_agent_when_older_timestamp() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: "◆ Thought 1.0s\nreasoning".into() }],
        timestamp: 2.0,
        id: "req.0#thought.0".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        parts: vec![Part::Text { content: "response".into() }],
        timestamp: 3.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds,
        vec!["Thought", "Agent"],
        "Thought(2.0) should naturally appear before Agent(3.0) — no fixup needed"
    );
}

#[test]
fn agent_before_thought_when_agent_newer() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: "◆ Thought 1.0s".into() }],
        timestamp: 5.0,
        id: "req.0#thought.0".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        parts: vec![Part::Text { content: "response".into() }],
        timestamp: 3.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(
        kinds,
        vec!["Agent", "Thought"],
        "Agent(3.0) must appear before Thought(5.0) when agent has older timestamp"
    );
}

#[test]
fn via_events_appended_assistant_found_anywhere_in_vec() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    state.update(Event::Response {
        id: "req.0".into(),
        content: "hello ".into(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::ToolStart { id: "req.0".into(), name: "ls".into(), input: serde_json::Value::Null });
    state.update(Event::ToolEnd { id: "".to_string(), input: None, duration_secs: 0.5, output: "file1".into() });
    // This next response should append to the SAME assistant message, not create a new one
    state.update(Event::Response {
        id: "req.0".into(),
        content: "world".into(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.ensure_fresh();

    let assistant_count = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .count();
    assert_eq!(
        assistant_count, 1,
        "Should not create duplicate assistant messages for same id"
    );
}
