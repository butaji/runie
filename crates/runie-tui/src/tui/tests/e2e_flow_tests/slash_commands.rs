use super::*;

#[test]
fn test_e2e_slash_clear() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add messages
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: None, timestamp: None });

    // Slash clear
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Clear));

    assert!(state.messages.is_empty());
}

#[test]
fn test_e2e_slash_new() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add messages
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });

    // Slash new - clears messages and adds system message
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::New));

    // Should have one system message "New session started"
    assert_eq!(state.messages.len(), 1);
    assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("New session")));
    assert_eq!(state.scroll.feed_offset, 0);
    assert!(state.scroll.feed_offset == 0);
    // Should have new session system message
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("New session"))));
}

#[test]
fn test_e2e_slash_model() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Slash model
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Model("gpt-4o".to_string())));

    assert_eq!(state.current_model.as_deref(), Some("gpt-4o"));
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Model switched"))));
}

#[test]
fn test_e2e_slash_help() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Slash help
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Help));

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("/"))));
}

#[test]
fn test_e2e_slash_unknown() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Unknown command
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Unknown("badcmd".to_string())));

    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Unknown command"))));
}
