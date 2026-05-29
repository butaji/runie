use super::*;

#[test]
fn test_clear_chat_resets_messages() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.messages.push(MessageItem::User { text: "hello".to_string(), model: Some("You".to_string()), timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "hi".to_string(), model: Some("gpt-4".to_string()), timestamp: None });
    state.messages.push(MessageItem::System { text: "system".to_string() });

    update(&mut state, &mut palette, Msg::ClearChat);

    assert!(state.messages.is_empty(), "Messages should be cleared");
}

#[test]
fn test_clear_chat_during_agent_run() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.agent_running = true;
    state.messages.push(MessageItem::User { text: "hello".to_string(), model: Some("You".to_string()), timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "hi".to_string(), model: Some("gpt-4".to_string()), timestamp: None });

    update(&mut state, &mut palette, Msg::ClearChat);

    assert!(state.messages.is_empty(), "Messages should be cleared");
    assert!(state.agent_running, "agent_running should remain true — agent continues running");
}

#[test]
fn test_clear_chat_resets_session_token_usage() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.session_token_usage.total_tokens = 1000;
    state.session_token_usage.prompt_tokens = 500;
    state.session_token_usage.completion_tokens = 500;
    state.session_token_usage.estimated_cost = 0.05;

    update(&mut state, &mut palette, Msg::ClearChat);

    // BUG-15 behavior: session_token_usage is NOT cleared by ClearChat
    assert_eq!(state.session_token_usage.total_tokens, 1000,
        "session_token_usage is NOT reset by ClearChat (documented behavior — may be a bug)");
}

// P1-REMAINING-1 FIX: Double-tap Ctrl+C to clear text
#[test]
fn test_clear_input_confirm_first_tap_shows_hint() {
    let mut state = make_state_with_text("Hello world");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::ClearInputConfirm);

    assert!(!state.textarea.is_empty(), "First tap should NOT clear text");
    assert!(state.input_right_info.contains("Ctrl+C again"),
        "First tap should show hint: {}", state.input_right_info);
}

#[test]
fn test_clear_input_confirm_second_tap_clears_text() {
    let mut state = make_state_with_text("Hello world");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::ClearInputConfirm);
    assert!(!state.textarea.is_empty(), "Text should not be cleared yet");

    update(&mut state, &mut palette, Msg::ClearInputConfirm);
    assert!(state.textarea.is_empty(), "Second tap should clear text");
    assert!(state.input_right_info.is_empty(), "Info should be cleared after clear");
}

#[test]
fn test_clear_input_confirm_timeout_resets() {
    let mut state = make_state_with_text("Hello world");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::ClearInputConfirm);

    state.clear_input_confirm.last_press = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(3)
    );

    update(&mut state, &mut palette, Msg::ClearInputConfirm);
    assert!(!state.textarea.is_empty(), "After timeout, next tap is first tap");
    assert!(state.input_right_info.contains("Ctrl+C again"),
        "After timeout, hint should show again");
}
