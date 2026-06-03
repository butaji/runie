use super::*;

#[test]
fn test_e2e_chat_submit_with_model() {
    let mut state = make_state_with_text("Hello, world!");
    let mut palette = CommandPalette::new();

    // Submit
    let cmds = update(&mut state, &mut palette, Msg::Submit);

    // Verify user message added + placeholder assistant
    assert_eq!(state.messages.len(), 2); // user + placeholder
    assert!(matches!(&state.messages[0], MessageItem::User { text, .. } if text == "Hello, world!"));

    // Verify agent spawned
    assert!(state.agent_running);
    assert!(matches!(&cmds[0], Cmd::SpawnAgent { .. }));
}

#[test]
fn test_e2e_chat_submit_empty_text_rejected() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Submit empty
    let cmds = update(&mut state, &mut palette, Msg::Submit);

    // Should not spawn agent
    assert!(cmds.is_empty());
    assert!(!state.agent_running);
    assert!(state.input_right_info.contains("Type a message"));
}

#[test]
fn test_e2e_chat_submit_no_model_shows_hint() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.textarea = ratatui_textarea::TextArea::new(vec!["Hello".to_string()]);

    // Submit without model
    let cmds = update(&mut state, &mut palette, Msg::Submit);

    // Should not spawn agent, show error
    assert!(cmds.is_empty());
    assert!(!state.agent_running);
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::Error { message, .. } if message.contains("No model"))));
}

#[test]
fn test_e2e_chat_submit_while_agent_running_cancels_and_proceeds() {
    let mut state = make_state_with_text("Hello!");
    let mut palette = CommandPalette::new();
    // First submit
    update(&mut state, &mut palette, Msg::Submit);
    assert_eq!(state.messages.len(), 2); // user + placeholder
    assert!(state.agent_running);

    // Submit again while agent running - should cancel old and proceed
    state.textarea = ratatui_textarea::TextArea::new(vec!["Second message".to_string()]);
    let cmds = update(&mut state, &mut palette, Msg::Submit);

    // Should proceed with new submit (old placeholder removed, adds user + placeholder)
    assert_eq!(state.messages.len(), 3); // user1 + placeholder1 + user2
    assert!(state.agent_running);
    assert!(!cmds.is_empty());
}

#[test]
fn test_e2e_clear_chat() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add some messages
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });

    // Clear chat
    update(&mut state, &mut palette, Msg::ClearChat);

    assert!(state.messages.is_empty());
}

#[test]
fn test_e2e_clear_input() {
    let mut state = make_state_with_text("Hello, world!");
    let mut palette = CommandPalette::new();

    // Clear input
    update(&mut state, &mut palette, Msg::ClearInput);

    // Textarea should be empty
    let text = state.textarea.lines().join("");
    assert!(text.is_empty());
}
