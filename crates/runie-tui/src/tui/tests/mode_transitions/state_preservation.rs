//! Tests for state preservation across mode transitions.
//!
//! Verifies that messages, textarea, scroll, input history, and token usage
//! are preserved when switching between modes.

use super::*;

/// Test: Messages preserved across Chat → Palette → Chat.
#[test]
fn test_messages_preserved_across_palette() {
    let mut state = make_state_with_messages(vec![
        MessageItem::User { text: "Hello".to_string(), model: Some("You".to_string()), timestamp: None },
        MessageItem::Assistant { text: "Hi there!".to_string(), model: Some("gpt-4".to_string()), timestamp: None, expanded: false },
    ]);
    let mut palette = CommandPalette::new();

    let original_messages = state.messages.clone();

    // Go to palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Return to chat
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);

    // Messages preserved
    assert_eq!(state.messages.len(), original_messages.len());
    assert_eq!(state.messages, original_messages);
}

/// Test: Textarea preserved across Chat → Palette → Chat.
#[test]
fn test_textarea_preserved_across_palette() {
    let mut state = make_state_with_text("typed input");
    let mut palette = CommandPalette::new();

    let original_input = state.textarea.clone();

    // Go to palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Return to chat
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);

    // Textarea preserved
    assert_eq!(state.textarea.lines(), original_input.lines());
}

/// Test: Scroll preserved across Chat → Palette → Chat.
#[test]
fn test_scroll_preserved_across_palette() {
    let mut state = make_state();
    state.scroll.feed_offset = 42;
    let mut palette = CommandPalette::new();

    let original_scroll = state.scroll.feed_offset;

    // Go to palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);

    // Return to chat
    update(&mut state, &mut palette, Msg::CloseModal);

    // Scroll preserved
    assert_eq!(state.scroll.feed_offset, original_scroll);
}

/// Test: Messages preserved across Chat → Overlay → Chat.
#[test]
fn test_messages_preserved_across_overlay() {
    let mut state = make_state_with_messages(vec![
        MessageItem::User { text: "Hello".to_string(), model: Some("You".to_string()), timestamp: None },
        MessageItem::Assistant { text: "Hi there!".to_string(), model: Some("gpt-4".to_string()), timestamp: None, expanded: false },
    ]);
    let mut palette = CommandPalette::new();

    let original_messages = state.messages.clone();

    // Go to overlay
    update(&mut state, &mut palette, Msg::SwitchModel);
    assert_eq!(state.mode, TuiMode::Overlay);

    // Return to chat
    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);

    // Messages preserved
    assert_eq!(state.messages.len(), original_messages.len());
    assert_eq!(state.messages, original_messages);
}

/// Test: Textarea preserved across Chat → Overlay → Chat.
#[test]
fn test_textarea_preserved_across_overlay() {
    let mut state = make_state_with_text("typed input");
    let mut palette = CommandPalette::new();

    let original_input = state.textarea.clone();

    // Go to overlay
    update(&mut state, &mut palette, Msg::SwitchModel);

    // Return to chat
    update(&mut state, &mut palette, Msg::CloseModal);

    // Textarea preserved
    assert_eq!(state.textarea.lines(), original_input.lines());
}

/// Test: Messages preserved across Chat → Permission → Chat.
#[test]
fn test_messages_preserved_across_permission() {
    let mut state = make_state_with_messages(vec![
        MessageItem::User { text: "Hello".to_string(), model: Some("You".to_string()), timestamp: None },
        MessageItem::Assistant { text: "Hi there!".to_string(), model: Some("gpt-4".to_string()), timestamp: None, expanded: false },
    ]);
    let mut palette = CommandPalette::new();

    let original_messages = state.messages.clone();

    // Go to permission
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_test".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.5,
    }));
    assert_eq!(state.mode, TuiMode::Permission);

    // Return to chat
    update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert_eq!(state.mode, TuiMode::Chat);

    // Messages preserved — the original entries must still be present
    // (preservation invariant).  New entries are allowed: a System
    // "Permission requested" message is appended when the modal opens so
    // the chat scrollback reflects what the agent is asking for.
    assert!(state.messages.len() >= original_messages.len());
    for original in &original_messages {
        assert!(
            state.messages.contains(original),
            "original message {original:?} not preserved across permission cycle"
        );
    }
}

/// Test: Textarea preserved across Chat → Permission → Chat.
#[test]
fn test_textarea_preserved_across_permission() {
    let mut state = make_state_with_text("typed input");
    let mut palette = CommandPalette::new();

    let original_input = state.textarea.clone();

    // Go to permission
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_test".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.5,
    }));

    // Return to chat
    update(&mut state, &mut palette, Msg::PermissionConfirm);

    // Textarea preserved
    assert_eq!(state.textarea.lines(), original_input.lines());
}

/// Test: Input history preserved across mode switches.
#[test]
fn test_input_history_preserved() {
    let mut state = make_state();
    state.input_history = vec!["first".to_string(), "second".to_string()];
    state.input_history_index = Some(1);
    let mut palette = CommandPalette::new();

    let original_history = state.input_history.clone();

    // Go to palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    update(&mut state, &mut palette, Msg::CloseModal);

    // History preserved
    assert_eq!(state.input_history, original_history);
    assert_eq!(state.input_history_index, Some(1));
}

/// Test: Token usage preserved across mode switches.
#[test]
fn test_token_usage_preserved() {
    let mut state = make_state();
    state.session_token_usage = runie_ai::TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 200,
        total_tokens: 300,
        estimated_cost: 0.01,
    };
    let mut palette = CommandPalette::new();

    let original_usage = state.session_token_usage.clone();

    // Go to palette and back
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    update(&mut state, &mut palette, Msg::CloseModal);

    // Token usage preserved
    assert_eq!(state.session_token_usage.total_tokens, original_usage.total_tokens);
    assert_eq!(state.session_token_usage.prompt_tokens, original_usage.prompt_tokens);
    assert_eq!(state.session_token_usage.completion_tokens, original_usage.completion_tokens);
}

/// Test: Scroll preserved across Permission mode.
#[test]
fn test_scroll_preserved_across_permission() {
    let mut state = make_state();
    state.scroll.feed_offset = 100;
    let mut palette = CommandPalette::new();

    let original_scroll = state.scroll.feed_offset;

    // Go to permission
    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_test".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
        tool_description: "List files".to_string(),
        turn: 1,
        context_window_usage: 0.5,
    }));

    // Return to chat
    update(&mut state, &mut palette, Msg::PermissionConfirm);

    // Scroll preserved
    assert_eq!(state.scroll.feed_offset, original_scroll);
}

/// Test: Messages preserved across Chat → Onboarding → Chat.
#[test]
fn test_messages_preserved_across_onboarding() {
    let mut state = make_state_with_messages(vec![
        MessageItem::User { text: "Hello".to_string(), model: Some("You".to_string()), timestamp: None },
        MessageItem::Assistant { text: "Hi there!".to_string(), model: Some("gpt-4".to_string()), timestamp: None, expanded: false },
    ]);
    let mut palette = CommandPalette::new();

    let original_messages = state.messages.clone();

    // Go to onboarding
    update(&mut state, &mut palette, Msg::EnterOnboarding);
    assert_eq!(state.mode, TuiMode::Onboarding);

    // Return to chat
    update(&mut state, &mut palette, Msg::OnboardingSkip);
    assert_eq!(state.mode, TuiMode::Chat);

    // Messages preserved
    assert_eq!(state.messages.len(), original_messages.len());
    assert_eq!(state.messages, original_messages);
}

/// Test: Messages preserved across Chat → SessionTree → Chat.
#[test]
fn test_messages_preserved_across_session_tree() {
    let mut state = make_state_with_messages(vec![
        MessageItem::User { text: "Hello".to_string(), model: Some("You".to_string()), timestamp: None },
        MessageItem::Assistant { text: "Hi there!".to_string(), model: Some("gpt-4".to_string()), timestamp: None, expanded: false },
    ]);
    let mut palette = CommandPalette::new();

    let original_messages = state.messages.clone();

    // Go to session tree
    update(&mut state, &mut palette, Msg::ToggleSessionTree);
    assert_eq!(state.mode, TuiMode::SessionTree);

    // Return to chat
    update(&mut state, &mut palette, Msg::ToggleSessionTree);
    assert_eq!(state.mode, TuiMode::Chat);

    // Messages preserved
    assert_eq!(state.messages.len(), original_messages.len());
    assert_eq!(state.messages, original_messages);
}

/// Test: All state preserved through multiple rapid mode switches.
#[test]
fn test_state_preserved_through_rapid_switches() {
    let mut state = make_state_with_text("typed input");
    state.scroll.feed_offset = 50;
    state.input_history = vec!["cmd1".to_string(), "cmd2".to_string()];
    state.session_token_usage = runie_ai::TokenUsage {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 37,
        estimated_cost: 0.001,
    };
    let mut palette = CommandPalette::new();

    // Rapid switches: Chat -> Palette -> Chat -> Overlay -> Chat
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    update(&mut state, &mut palette, Msg::CloseModal);
    update(&mut state, &mut palette, Msg::SwitchModel);
    update(&mut state, &mut palette, Msg::CloseModal);

    // All state preserved
    assert_eq!(state.textarea.lines(), ["typed input"]);
    assert_eq!(state.scroll.feed_offset, 50);
    assert_eq!(state.input_history.len(), 2);
    assert_eq!(state.session_token_usage.total_tokens, 37);
}
