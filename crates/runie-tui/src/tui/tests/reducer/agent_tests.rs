use super::*;

#[test]
fn test_agent_event_message_start() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    update(
        &mut state,
        &mut palette,
        Msg::AgentEvent(AgentEvent::MessageStart {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
                tool_calls: vec![],
            },
            turn: 1,
        }),
    );
    assert!(state.agent_running);
    assert_eq!(state.messages.len(), 1);
}

fn create_message_start_event(turn: usize) -> AgentEvent {
    AgentEvent::MessageStart {
        message: AgentMessage {
            role: "assistant".to_string(),
            content: vec![],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        },
        turn,
    }
}

fn create_message_update_event(turn: usize, text: &str) -> AgentEvent {
    use runie_agent::ContentPart;
    AgentEvent::MessageUpdate {
        message: AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text {
                text: text.to_string(),
            }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        },
        delta: text.to_string(),
        replace: true,
        turn,
    }
}

#[test]
fn test_agent_event_message_update() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::AgentEvent(create_message_start_event(1)));
    update(&mut state, &mut palette, Msg::AgentEvent(create_message_update_event(1, "Hello")));

    assert_eq!(state.messages.len(), 1);
    if let MessageItem::Assistant { text, .. } = &state.messages[0] {
        assert_eq!(text, "Hello");
    } else {
        panic!("Expected Assistant message");
    }
}

// P0-1 FIX: Msg::Stop interrupts agent without quitting
#[test]
fn test_msg_stop_clears_agent_running() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.agent_running = true;
    state.mode = TuiMode::Permission;

    let cmds = update(&mut state, &mut palette, Msg::Stop);

    assert!(!state.agent_running, "agent_running should be cleared on Stop");
    assert_eq!(state.mode, TuiMode::Chat, "Mode should reset to Chat on Stop (not Onboarding)");
    assert!(state.running, "running should remain true on Stop (Quit sets it false)");

    assert!(!cmds.is_empty(), "Stop should produce at least one cmd");
    if let Cmd::Interrupt = &cmds[0] {
        // Expected
    } else {
        panic!("Expected Cmd::Interrupt");
    }
}

// BG-2 FIX: Agent error resets mode to Chat
#[test]
fn test_agent_error_resets_mode() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.mode = TuiMode::Permission;

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::Error {
        message: "Connection reset".to_string(),
        error_type: "network".to_string(),
        recoverable: true,
        context: "test".to_string(),
    }));

    assert_eq!(state.mode, TuiMode::Chat, "Mode should reset to Chat on agent error");
}

#[test]
fn test_long_error_is_truncated() {
    use crate::tui::update::agent::sanitize_error_message;

    let long_error = "Error: ".to_string() + &"x".repeat(1000);
    let sanitized = sanitize_error_message(&long_error);

    assert!(sanitized.len() < long_error.len(), "Long error should be truncated");
    assert!(sanitized.contains("[message truncated"), "Should indicate truncation");
}

#[test]
fn test_stack_trace_shows_summary() {
    use crate::tui::update::agent::sanitize_error_message;

    let stack_trace = "Connection error\nstack backtrace:\n   at 0x7f8d9f... (main.rs:100)\n   at 0x7f8da0... (main.rs:101)";
    let sanitized = sanitize_error_message(stack_trace);

    // The sanitizer keeps the first line (the actual error) and appends a
    // compact "[hidden]" marker.  Test invariants:
    //   1. the error summary is preserved
    //   2. a "hidden" note is present so the user knows the backtrace was elided
    assert!(sanitized.contains("Connection error"), "Should preserve error summary");
    assert!(sanitized.contains("hidden"), "Should add hidden details note");
    assert!(
        sanitized.len() < stack_trace.len(),
        "Sanitized output should be strictly shorter than the input ({} >= {})",
        sanitized.len(),
        stack_trace.len()
    );
}

#[test]
fn test_error_messages_filtered_from_agent_context() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.current_model = Some("gpt-4".to_string());

    state.textarea = TextArea::new(vec!["hello".to_string()]);
    update(&mut state, &mut palette, Msg::Submit);
    assert_eq!(state.messages.len(), 2); // user + placeholder assistant

    state.agent_running = false;

    state.messages.push(MessageItem::Error { message: "Something went wrong".to_string(), recoverable: false });

    state.textarea = TextArea::new(vec!["world".to_string()]);
    let cmds = update(&mut state, &mut palette, Msg::Submit);

    assert_eq!(cmds.len(), 1);
    if let crate::tui::state::Cmd::SpawnAgent { messages } = &cmds[0] {
        let roles: Vec<_> = messages.iter().map(|m| m.role.as_str()).collect();
        assert!(!roles.contains(&"error"), "Error message should not be in agent messages");
        assert!(roles.contains(&"user"));
    }
}
