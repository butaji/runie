use super::*;

#[test]
fn test_e2e_palette_open_filter_confirm() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);
    assert!(state.command_palette.open);

    // Type filter
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('q'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('u'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('i'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('t'));
    assert_eq!(state.command_palette.filter, "quit");

    // Confirm (Quit)
    let cmds = update(&mut state, &mut palette, Msg::CommandPaletteConfirm);
    assert!(cmds.iter().any(|c| matches!(c, Cmd::Interrupt)));
    assert!(!state.running);
}

#[test]
fn test_e2e_palette_cancel() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Cancel with Escape
    update(&mut state, &mut palette, Msg::CommandPaletteCancelArgument);
    assert_eq!(state.mode, TuiMode::Chat);
    assert!(!state.command_palette.open);
}

#[test]
fn test_e2e_palette_navigation() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    let initial_selected = state.command_palette.selected;

    // Navigate down
    update(&mut state, &mut palette, Msg::CommandPaletteDown);
    assert_eq!(state.command_palette.selected, initial_selected + 1);

    // Navigate up
    update(&mut state, &mut palette, Msg::CommandPaletteUp);
    assert_eq!(state.command_palette.selected, initial_selected);
}

#[test]
fn test_e2e_palette_clear_chat_command() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add a message
    state.messages.push(MessageItem::User { text: "Test".to_string(), model: None, timestamp: None });

    // Open palette and filter for clear
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('c'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('l'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('e'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('a'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('r'));

    // Confirm clear - adds system message "Chat cleared"
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);
    assert_eq!(state.messages.len(), 1);
    assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("cleared")));
}

#[test]
fn test_e2e_palette_backspace_filter() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('q'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('u'));
    assert_eq!(state.command_palette.filter, "qu");

    // Backspace
    update(&mut state, &mut palette, Msg::CommandPaletteBackspace);
    assert_eq!(state.command_palette.filter, "q");
}
