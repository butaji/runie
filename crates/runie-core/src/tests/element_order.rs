use crate::model::{AppState, ChatMessage, Role};
use crate::ui::LazyCache;
use crate::Event;

fn element_kinds(state: &AppState) -> Vec<String> {
    let feed = LazyCache::feed(state);
    feed.elements.iter().map(|e| match e {
        crate::ui::Element::UserMessage { .. } => "User".to_string(),
        crate::ui::Element::AgentMessage { .. } => "Agent".to_string(),
        crate::ui::Element::Thinking { .. } => "Thinking".to_string(),
        crate::ui::Element::ThoughtMarker { .. } => "Thought".to_string(),
        crate::ui::Element::ThoughtSummary { .. } => "ThoughtSum".to_string(),
        crate::ui::Element::ToolRunning { .. } => "ToolRun".to_string(),
        crate::ui::Element::ToolDone { .. } => "ToolDone".to_string(),
        crate::ui::Element::ToolSummary { .. } => "ToolSum".to_string(),
        crate::ui::Element::TurnComplete { .. } => "Turn".to_string(),
        crate::ui::Element::Spacer { .. } => "Spacer".to_string(),
    }).collect()
}

fn element_kinds_no_spacer(state: &AppState) -> Vec<String> {
    element_kinds(state).into_iter().filter(|k| k != "Spacer").collect()
}

#[test]
fn elements_ordered_by_timestamp_strict() {
    let mut state = AppState::default();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ ls 0.5s\noutput".into(),
        timestamp: 2.0,
        id: "tool.req.0.1".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "world".into(),
        timestamp: 3.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s\nhello".into(),
        timestamp: 4.0,
        id: "req.0#thought.0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds, vec!["ToolDone", "Agent", "Thought"],
        "Elements must be strictly ordered by timestamp: Tool(2.0) < Agent(3.0) < Thought(4.0)");
}

#[test]
fn newer_assistant_appears_after_older_thought() {
    let mut state = AppState::default();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s".into(),
        timestamp: 1.0,
        id: "req.0#thought.0".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "updated later".into(),
        timestamp: 5.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds, vec!["Thought", "Agent"],
        "Thought(1.0) must appear before Agent(5.0) by timestamp");
}

#[test]
fn thinking_indicator_is_always_last_when_newest() {
    let mut state = AppState::default();
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".into(),
        timestamp: 1.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "hi".into(),
        timestamp: 2.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.thinking_started_at = Some(std::time::Instant::now());
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds, vec!["User", "Agent", "Thinking"],
        "Thinking indicator (max_ts+1) must be last");
}

#[test]
fn streaming_bump_moves_assistant_to_end() {
    let mut state = AppState::default();
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "Q1".into(),
        timestamp: 1.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "A1".into(),
        timestamp: 2.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "Q2".into(),
        timestamp: 3.0,
        id: "u1".into(),
        ..Default::default()
    });
    // Now bump assistant timestamp to 4.0 — simulating streaming update
    if let Some(msg) = state.messages.iter_mut().find(|m| m.role == Role::Assistant && m.id == "req.0") {
        msg.timestamp = 4.0;
    }
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds, vec!["User", "User", "Agent"],
        "Agent bumped to 4.0 must appear after User at 3.0");
}

#[test]
fn tool_end_bump_moves_tool_after_later_messages() {
    let mut state = AppState::default();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "Running ls...".into(),
        timestamp: 2.0,
        id: "tool.req.0.1".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "next".into(),
        timestamp: 3.0,
        id: "u1".into(),
        ..Default::default()
    });
    // Tool completes — bump to 5.0
    if let Some(msg) = state.messages.iter_mut().find(|m| m.role == Role::Tool) {
        msg.timestamp = 5.0;
        msg.content = "✓ ls 0.5s\noutput".into();
    }
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds, vec!["User", "ToolDone"],
        "Tool bumped to 5.0 must appear after User at 3.0");
}

#[test]
fn multiple_tools_ordered_by_completion_time() {
    let mut state = AppState::default();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ cat 0.1s".into(),
        timestamp: 5.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ ls 0.2s".into(),
        timestamp: 2.0,
        id: "t2".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "✓ grep 0.3s".into(),
        timestamp: 8.0,
        id: "t3".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds, vec!["ToolDone", "ToolDone", "ToolDone"],
        "Tools should be ordered by timestamp");
    // Also verify the actual order by checking content
    let feed = LazyCache::feed(&state);
    let texts: Vec<String> = feed.elements.iter().filter_map(|e| match e {
        crate::ui::Element::ToolDone { name, .. } => Some(name.clone()),
        _ => None,
    }).collect();
    assert_eq!(texts, vec!["ls", "cat", "grep"],
        "Tool order must follow timestamp: ls(2) < cat(5) < grep(8)");
}

#[test]
fn thought_before_agent_when_older_timestamp() {
    let mut state = AppState::default();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s\nreasoning".into(),
        timestamp: 2.0,
        id: "req.0#thought.0".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "response".into(),
        timestamp: 3.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds, vec!["Thought", "Agent"],
        "Thought(2.0) should naturally appear before Agent(3.0) — no fixup needed");
}

#[test]
fn agent_before_thought_when_agent_newer() {
    let mut state = AppState::default();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.0s".into(),
        timestamp: 5.0,
        id: "req.0#thought.0".into(),
        ..Default::default()
    });
    state.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "response".into(),
        timestamp: 3.0,
        id: "req.0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let kinds = element_kinds_no_spacer(&state);
    assert_eq!(kinds, vec!["Agent", "Thought"],
        "Agent(3.0) must appear before Thought(5.0) when agent has older timestamp");
}

#[test]
fn via_events_appended_assistant_found_anywhere_in_vec() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "hello ".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1".into() });
    // This next response should append to the SAME assistant message, not create a new one
    state.update(Event::AgentResponse { id: "req.0".into(), content: "world".into() });
    state.ensure_fresh();

    let assistant_count = state.messages.iter().filter(|m| m.role == Role::Assistant).count();
    assert_eq!(assistant_count, 1, "Should not create duplicate assistant messages for same id");
}
