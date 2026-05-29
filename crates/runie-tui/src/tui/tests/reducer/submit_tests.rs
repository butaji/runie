use super::*;

#[test]
fn test_submit_clears_input() {
    let mut state = make_state_with_text("hi");
    let mut palette = CommandPalette::new();
    let cmds = update(&mut state, &mut palette, Msg::Submit);
    assert!(state.textarea.is_empty());
    assert_eq!(state.messages.len(), 2); // user + placeholder assistant
    assert_eq!(cmds.len(), 1);
    if let crate::tui::state::Cmd::SpawnAgent { .. } = &cmds[0] {
        // Expected
    } else {
        panic!("Expected SpawnAgent cmd");
    }
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "hi");
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_submit_empty_does_nothing() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    let cmds = update(&mut state, &mut palette, Msg::Submit);
    assert_eq!(state.messages.len(), 0);
    assert!(cmds.is_empty());
}

#[test]
fn test_multi_line_submit() {
    let mut state = make_state();
    state.current_model = Some("gpt-4".to_string());
    let mut palette = CommandPalette::new();
    for c in "line1".chars() {
        type_char(&mut state, c);
    }
    type_enter(&mut state);
    for c in "line2".chars() {
        type_char(&mut state, c);
    }
    update(&mut state, &mut palette, Msg::Submit);

    assert!(state.textarea.is_empty());
    assert_eq!(state.messages.len(), 2); // user + placeholder assistant
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "line1\nline2");
    } else {
        panic!("Expected User message");
    }
}

// Submit empty text
#[test]
fn test_submit_empty_text_blocked() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    let cmds = update(&mut state, &mut palette, Msg::Submit);
    assert_eq!(state.messages.len(), 0);
    assert!(cmds.is_empty());
    assert_eq!(state.input_right_info, "Type a message first");
}

// Submit while agent running - cancels old agent and proceeds with new submit
#[test]
fn test_submit_while_agent_running_cancels_and_proceeds() {
    let mut state = make_state_with_text("Hello");
    let mut palette = CommandPalette::new();
    // First submit
    update(&mut state, &mut palette, Msg::Submit);
    assert_eq!(state.messages.len(), 2); // user + placeholder
    assert!(state.agent_running);

    // Second submit while agent running - should cancel old and proceed
    state.textarea = TextArea::new(vec!["Second message".to_string()]);
    let cmds = update(&mut state, &mut palette, Msg::Submit);

    // Should proceed with new submit (removes old placeholder, adds user + placeholder)
    assert_eq!(state.messages.len(), 3); // user1 + placeholder1 + user2 (old placeholder removed)
    assert!(state.agent_running);
    // Should have SpawnAgent command for the new submit
    assert!(!cmds.is_empty());
}

// Submit no model configured
#[test]
fn test_submit_no_model_configured() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.current_model = None;
    state.onboarding = None;
    state.textarea = TextArea::new(vec!["hello".to_string()]);
    let cmds = update(&mut state, &mut palette, Msg::Submit);
    assert_eq!(state.messages.len(), 3); // user + placeholder + error
    assert!(cmds.is_empty());
    if let MessageItem::System { text } = &state.messages[2] {
        assert!(text.contains("No model configured"));
    } else {
        panic!("Expected System message");
    }
}

// Submit while agent running - new behavior cancels old and proceeds
#[test]
fn test_submit_while_agent_running_feedback() {
    let mut state = make_state_with_text("Hello");
    let mut palette = CommandPalette::new();
    // Simulate agent already running
    state.agent_running = true;

    update(&mut state, &mut palette, Msg::Submit);

    // New behavior: submit proceeds (old agent is cancelled, new one starts)
    // Feedback may mention cancellation
    assert!(state.messages.len() >= 2, "Should add user message and placeholder");
    assert!(state.agent_running, "Agent should still be running with new submit");
}

// Submit while agent running - new behavior cancels old and proceeds with new submit
#[test]
fn test_duplicate_submit_cancels_and_proceeds() {
    let mut state = make_state_with_text("Hello");
    let mut palette = CommandPalette::new();

    update(&mut state, &mut palette, Msg::Submit);
    assert_eq!(state.messages.len(), 2, "First submit should add user + placeholder");

    state.textarea = TextArea::new(vec!["Second message".to_string()]);
    state.agent_running = true;

    // New behavior: cancels old agent, proceeds with new submit
    let cmds = update(&mut state, &mut palette, Msg::Submit);

    // Should have 3 messages now (user1 + placeholder1 + user2, old placeholder removed)
    assert_eq!(state.messages.len(), 3);
    // Should spawn new agent
    assert!(!cmds.is_empty());
}
