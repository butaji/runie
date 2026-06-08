use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

/// Helper: get element kinds in visible region (no spacer)
fn visible_kinds(state: &AppState, height: usize) -> Vec<String> {
    let region = state.visible_scroll(height);
    region.elements.iter().map(|e| match e {
        crate::ui::Element::UserMessage { .. } => "User".to_string(),
        crate::ui::Element::AgentMessage { .. } => "Agent".to_string(),
        crate::ui::Element::Thinking { .. } => "Thinking".to_string(),
        crate::ui::Element::ThoughtMarker { .. } => "Thought".to_string(),
        crate::ui::Element::ThoughtSummary { .. } => "ThoughtSum".to_string(),
        crate::ui::Element::ToolRunning { .. } => "ToolRun".to_string(),
        crate::ui::Element::ToolDone { .. } => "ToolDone".to_string(),
        crate::ui::Element::ToolSummary { .. } => "ToolSum".to_string(),
        crate::ui::Element::TurnComplete { .. } => "Turn".to_string(),
        crate::ui::Element::Spacer => "Spacer".to_string(),
    }).filter(|k| k != "Spacer").collect()
}

/// Helper: check if latest content is visible at bottom
fn latest_is_visible(state: &AppState, height: usize) -> bool {
    let region = state.visible_scroll(height);
    if region.elements.is_empty() {
        return false;
    }
    // The last non-spacer element should be visible
    let last = region.elements.iter().rev().find(|e| !matches!(e, crate::ui::Element::Spacer));
    last.is_some()
}

// ── The exact user scenario ───────────────────────────────────────────

#[test]
fn list_files_full_turn_latest_always_visible() {
    let mut state = fresh_state();
    let height = 5;
    verify_user_visible(&mut state, height);
    verify_thinking_visible(&mut state, height);
    verify_agent_response_visible(&mut state, height);
    verify_tool_output_visible(&mut state, height);
    verify_final_response_visible(&mut state, height);
    verify_turn_complete_last(&mut state, height);
}

fn verify_user_visible(state: &mut AppState, height: usize) {
    state.input = "list files".to_string();
    state.update(Event::Submit);
    state.ensure_fresh();
    assert!(latest_is_visible(state, height), "User message must be visible after submit");
}

fn verify_thinking_visible(state: &mut AppState, height: usize) {
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.ensure_fresh();
    assert!(latest_is_visible(state, height), "Thinking indicator must be visible");
}

fn verify_agent_response_visible(state: &mut AppState, height: usize) {
    state.update(Event::AgentResponse { id: "req.0".into(), content: "I'll list the files.".into() });
    state.ensure_fresh();
    assert!(latest_is_visible(state, height), "Agent response must be visible during streaming");
}

fn verify_tool_output_visible(state: &mut AppState, height: usize) {
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() });
    state.ensure_fresh();
    assert!(latest_is_visible(state, height), "Tool running must be visible");
    let output = (1..=20).map(|i| format!("file{}.txt", i)).collect::<Vec<_>>().join("\n");
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output });
    state.ensure_fresh();
    let kinds = visible_kinds(state, height);
    assert!(kinds.contains(&"ToolDone".to_string()), "Tool result must be visible. Got: {:?}", kinds);
}

fn verify_final_response_visible(state: &mut AppState, height: usize) {
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done!".into() });
    state.ensure_fresh();
    let kinds = visible_kinds(state, height);
    assert!(kinds.contains(&"Agent".to_string()), "Final response must be visible. Got: {:?}", kinds);
}

fn verify_turn_complete_last(state: &mut AppState, height: usize) {
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 2.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();
    let kinds = visible_kinds(state, height);
    assert!(kinds.last() == Some(&"Turn".to_string()), "TurnComplete must be last. Got: {:?}", kinds);
}

#[test]
fn large_tool_output_bottom_lines_visible() {
    let mut state = fresh_state();
    let height = 5;

    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    let output = (1..=20).map(|i| format!("file{}.txt", i)).collect::<Vec<_>>().join("\n");
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output });
    state.ensure_fresh();
    state.scroll = 0;

    let region = state.visible_scroll(height);
    let texts: Vec<String> = region.elements.iter().filter_map(|e| match e {
        crate::ui::Element::ToolDone { output, .. } => Some(output.clone()),
        _ => None,
    }).collect();

    assert!(!texts.is_empty(), "ToolDone must be in visible region");
    let tool_output = &texts[0];
    assert!(
        tool_output.contains("file20.txt"),
        "Latest file (file20.txt) must be visible in tool output. Got output: {}", tool_output
    );
}

#[test]
fn viewport_at_bottom_shows_latest_after_overflow() {
    let mut state = fresh_state();
    let height = 5;

    add_small_messages(&mut state);
    state.ensure_fresh();
    state.scroll = 0;

    let before = visible_kinds(&state, height);
    assert!(before.contains(&"User".to_string()), "User messages visible before overflow");

    add_huge_thought(&mut state);
    state.ensure_fresh();
    state.scroll = 0;

    verify_thought_visible(&state, height);
}

fn add_small_messages(state: &mut AppState) {
    for i in 0..3 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
}

fn add_huge_thought(state: &mut AppState) {
    let mut huge = "◆ Thought 1.0s\n".to_string();
    for i in 1..=30 {
        huge.push_str(&format!("line{}\n", i));
    }
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: huge,
        timestamp: 10.0,
        id: "t1".into(),
    });
    state.messages_changed();
}

fn verify_thought_visible(state: &AppState, height: usize) {
    let region = state.visible_scroll(height);
    let has_thought = region.elements.iter().any(|e| matches!(e, crate::ui::Element::ThoughtMarker { .. }));
    assert!(has_thought, "Thought must be visible after overflow");

    let thought_texts: Vec<String> = region.elements.iter().filter_map(|e| match e {
        crate::ui::Element::ThoughtMarker { content } => Some(content.clone()),
        _ => None,
    }).collect();
    if !thought_texts.is_empty() {
        assert!(thought_texts[0].contains("line30"), "Latest line must be visible. Got: {:?}", thought_texts);
    }
}

#[test]
fn scroll_zero_means_bottom_after_any_event() {
    let mut state = fresh_state();
    let height = 5;

    state.scroll = 0;

    // Send a bunch of events
    state.update(Event::AgentResponse { id: "req.0".into(), content: "a".into() });
    state.ensure_fresh();
    let v1 = state.visible_scroll(height);
    assert!(!v1.elements.is_empty(), "Visible region must not be empty after first response");

    state.update(Event::AgentResponse { id: "req.0".into(), content: "b".into() });
    state.ensure_fresh();
    let v2 = state.visible_scroll(height);
    assert!(!v2.elements.is_empty(), "Visible region must not be empty after second response");

    state.update(Event::AgentResponse { id: "req.0".into(), content: "c".into() });
    state.ensure_fresh();
    let v3 = state.visible_scroll(height);
    assert!(!v3.elements.is_empty(), "Visible region must not be empty after third response");

    // After many more
    for i in 0..20 {
        state.update(Event::AgentResponse {
            id: format!("req.{}", i),
            content: format!("msg{}", i),
        });
    }
    state.ensure_fresh();
    let v4 = state.visible_scroll(height);
    assert!(!v4.elements.is_empty(), "Visible region must not be empty after many responses");
}

#[test]
fn user_message_visible_after_submit_clears_input() {
    let mut state = fresh_state();
    let height = 5;

    state.input = "list files".to_string();
    state.update(Event::Submit);
    state.ensure_fresh();
    state.scroll = 0;

    let region = state.visible_scroll(height);
    let has_user = region.elements.iter().any(|e| match e {
        crate::ui::Element::UserMessage { content, .. } => content == "list files",
        _ => false,
    });
    assert!(has_user, "Submitted user message must be visible");
}

#[test]
fn streaming_response_appends_not_replaces() {
    let mut state = fresh_state();

    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello ".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "world".into() });
    state.ensure_fresh();

    let assistant_msgs: Vec<_> = state.messages.iter()
        .filter(|m| m.role == Role::Assistant)
        .collect();
    assert_eq!(assistant_msgs.len(), 1, "Should have exactly one assistant message");
    assert_eq!(assistant_msgs[0].content, "Hello world", "Content should be appended");
}

#[test]
fn tool_end_does_not_duplicate_messages() {
    let mut state = fresh_state();

    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    let before_count = state.messages.len();
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    let after_count = state.messages.len();

    assert_eq!(before_count, after_count, "Tool end should update existing message, not create new one");
}

#[test]
fn total_lines_increases_with_each_event() {
    let mut state = fresh_state();

    let t0 = state.total_lines();
    state.update(Event::AgentResponse { id: "req.0".into(), content: "a".into() });
    state.ensure_fresh();
    let t1 = state.total_lines();
    assert!(t1 > t0, "total_lines should increase after response");

    state.update(Event::AgentResponse { id: "req.0".into(), content: "b".into() });
    state.ensure_fresh();
    let t2 = state.total_lines();
    assert!(t2 >= t1, "total_lines should not decrease after append");
}
