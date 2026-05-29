use super::*;

#[test]
fn test_e2e_agent_message_start_end() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Simulate agent starting to respond
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::MessageStart {
        message: make_agent_message("assistant", ""),
        turn: 1,
    }));

    assert!(state.agent_running);
    assert_eq!(state.messages.len(), 1);
    assert!(matches!(&state.messages[0], MessageItem::Assistant { text, .. } if text.is_empty()));

    // Simulate message content
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::MessageUpdate {
        message: make_agent_message("assistant", "Hello"),
        turn: 1,
        delta: "Hello".to_string(),
    }));

    // Simulate message end
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::MessageEnd {
        message: make_agent_message("assistant", "Hello"),
        turn: 1,
    }));

    // agent_running remains true after MessageEnd - only AgentEnd clears it
    assert!(state.agent_running);

    // Simulate agent end
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    }));

    assert!(!state.agent_running);
}

#[test]
fn test_e2e_agent_error_sets_recoverable() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Simulate recoverable error
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::Error {
        message: "timeout: connection refused".to_string(),
        error_type: "network".to_string(),
        recoverable: true,
        context: "test".to_string(),
    }));

    assert!(!state.agent_running);
    assert_eq!(state.messages.len(), 1);
    assert!(matches!(&state.messages[0], MessageItem::Error { message: _, recoverable: true }));
}

#[test]
fn test_e2e_agent_error_sets_fatal() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Simulate fatal error
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::Error {
        message: "invalid_api_key".to_string(),
        error_type: "auth".to_string(),
        recoverable: false,
        context: "test".to_string(),
    }));

    assert!(!state.agent_running);
    assert!(matches!(&state.messages[0], MessageItem::Error { recoverable: false, .. }));
    assert_eq!(state.mode, TuiMode::Chat);
}

#[test]
fn test_e2e_agent_end_clears_running_flag() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();
    state.agent_running = true;
    state.agent_start_time = Some(std::time::Instant::now());

    // Simulate agent end
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    }));

    assert!(!state.agent_running);
    assert!(state.agent_start_time.is_none());
}

#[test]
fn test_e2e_agent_token_usage_accumulates() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Initial token usage should be zero
    assert_eq!(state.session_token_usage.total_tokens, 0);

    // Simulate token usage event
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
        context_window: 128000,
    }));

    assert_eq!(state.session_token_usage.prompt_tokens, 100);
    assert_eq!(state.session_token_usage.completion_tokens, 50);
    assert_eq!(state.session_token_usage.total_tokens, 150);
}
