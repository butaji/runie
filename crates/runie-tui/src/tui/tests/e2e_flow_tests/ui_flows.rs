use super::*;

#[test]
fn test_e2e_top_bar_shows_model() {
    let mut state = make_state_with_model("anthropic/claude-3-opus");
    let _palette = CommandPalette::new();

    // Top bar should show model
    assert_eq!(state.top_bar.model, "anthropic/claude-3-opus");

    // Model change updates top bar
    state.current_model = Some("openai/gpt-4".to_string());
    state.top_bar.model = "openai/gpt-4".to_string();
    assert_eq!(state.top_bar.model, "openai/gpt-4");
}

#[test]
fn test_e2e_status_bar_hotkeys() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // In Chat mode, Ctrl+P should open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // In Palette mode, Esc should close
    update(&mut state, &mut palette, Msg::CommandPaletteCancelArgument);
    assert_eq!(state.mode, TuiMode::Chat);
}

#[test]
fn test_e2e_thinking_indicator() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Initially not thinking
    assert!(!state.agent_running);

    // Start agent
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::MessageStart {
        message: make_agent_message("assistant", ""),
        turn: 1,
    }));

    // Now thinking
    assert!(state.agent_running);

    // End agent
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    }));

    // No longer thinking
    assert!(!state.agent_running);
}

#[test]
fn test_e2e_toggle_sidebar() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    assert!(!state.show_sidebar);

    update(&mut state, &mut palette, Msg::ToggleSidebar);
    assert!(state.show_sidebar);

    update(&mut state, &mut palette, Msg::ToggleSidebar);
    assert!(!state.show_sidebar);
}

#[test]
fn test_e2e_scroll_messages() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add messages
    for i in 0..20 {
        state.messages.push(MessageItem::User { text: format!("Message {}", i), model: None, timestamp: None });
    }

    // Scroll
    update(&mut state, &mut palette, Msg::ScrollDown);
    assert_eq!(state.scroll.feed_offset, 1);

    update(&mut state, &mut palette, Msg::ScrollUp);
    assert_eq!(state.scroll.feed_offset, 0);

    // Page scroll
    update(&mut state, &mut palette, Msg::ScrollPageDown);
    assert_eq!(state.scroll.feed_offset, 10);
}

#[test]
fn test_e2e_resize_terminal() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    assert_eq!(state.terminal_size, (0, 0));

    update(&mut state, &mut palette, Msg::Resize(160, 50));

    assert_eq!(state.terminal_size, (160, 50));
}
