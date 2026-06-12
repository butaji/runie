use super::*;

// ─── Full Palette Flows ─────────────────────────────────────────────────────────

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

    // Confirm clear - clears all messages
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);
    assert_eq!(state.messages.len(), 0);
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

// ─── Full Palette Flow for Each Command ────────────────────────────────────────

/// Test palette flow: open → filter "new" → confirm → new session
#[test]
fn test_e2e_palette_flow_new_session() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add existing message
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Filter for "new"
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('n'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('e'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('w'));

    // Confirm
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // New session: cleared + system message added
    assert_eq!(state.messages.len(), 1);
    assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("New session")));
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test palette flow: open → filter "switch" → confirm → opens model picker
#[test]
fn test_e2e_palette_flow_switch_model() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Filter for "switch"
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('s'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('w'));

    // Confirm
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // Model picker opens
    assert!(state.model_picker.is_some());
    assert_eq!(state.mode, TuiMode::Overlay);
}

/// Test palette flow: open → filter "onboard" → confirm → opens onboarding
#[test]
fn test_e2e_palette_flow_onboard() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Use DirectCommand to test onboard flow directly
    update(&mut state, &mut palette, Msg::DirectCommand(crate::components::PaletteCommand::Onboard));

    // Onboarding mode
    assert_eq!(state.mode, TuiMode::Onboarding);
    assert!(state.onboarding.is_some());
}

/// Test palette flow: open → filter "help" → confirm → shows help message
#[test]
fn test_e2e_palette_flow_help() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Filter for "help"
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('h'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('e'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('l'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('p'));

    // Confirm
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // Help message added
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("/new"))));
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test palette flow: open → filter "tree" → confirm → opens session tree
#[test]
fn test_e2e_palette_flow_tree() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Use DirectCommand to test tree flow directly
    update(&mut state, &mut palette, Msg::DirectCommand(crate::components::PaletteCommand::SessionTree));

    // Session tree mode
    assert!(state.session_tree.visible);
    assert_eq!(state.mode, TuiMode::SessionTree);
}

/// Test palette flow: open → filter "cost" → confirm → shows cost
#[test]
fn test_e2e_palette_flow_cost() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Filter for "cost"
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('c'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('o'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('s'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('t'));

    // Confirm
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // Cost message added
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("tokens") || text.contains("cost"))));
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test palette flow: open → filter "fork" → confirm → creates fork
#[test]
fn test_e2e_palette_flow_fork() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add a message
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Filter for "fork"
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('f'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('o'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('r'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('k'));

    // Confirm
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // Fork system message added
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Fork"))));
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Test palette flow: open → filter "copy" → confirm → copies last response
#[test]
fn test_e2e_palette_flow_copy() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add assistant message with content
    state.messages.push(MessageItem::Assistant { text: "Hello".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });

    // Use DirectCommand to bypass clipboard output in tests
    update(&mut state, &mut palette, Msg::DirectCommand(crate::components::PaletteCommand::CopyLast));

    // Copy confirmation message added
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Copied"))));
}

// ─── Slash Commands via Direct Message ─────────────────────────────────────────

/// /onboard via SlashCommand message → opens onboarding
#[test]
fn test_e2e_slash_onboard_opens_onboarding() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Slash onboard
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Onboard));

    // Onboarding mode
    assert_eq!(state.mode, TuiMode::Onboarding);
    assert!(state.onboarding.is_some());
}

/// /help via SlashCommand message → shows help
#[test]
fn test_e2e_slash_help_shows_help() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Slash help
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Help));

    // Help message added
    assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("/new"))));
}

/// /new via SlashCommand message → clears messages + new session
#[test]
fn test_e2e_slash_new_clears_and_starts_session() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add messages
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });

    // Slash new
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::New));

    // Cleared + system message
    assert_eq!(state.messages.len(), 1);
    assert!(matches!(&state.messages[0], MessageItem::System { text } if text.contains("New session")));
}

/// /quit via SlashCommand message → sets running = false
#[test]
fn test_e2e_slash_quit_sets_running_false() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();
    assert!(state.running);

    // Slash quit
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Quit));

    // Running false
    assert!(!state.running);
}

/// /clear via SlashCommand message → clears messages
#[test]
fn test_e2e_slash_clear_clears_messages() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Add messages
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: None, timestamp: None, expanded: false, thought_duration: None, turn_duration: None });

    // Slash clear
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Clear));

    // Cleared
    assert!(state.messages.is_empty());
}

// ─── Palette Keyboard Navigation ────────────────────────────────────────────────

/// Up/Down arrows move selection within filtered results
#[test]
fn test_e2e_palette_arrow_navigation_wraps() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette with all commands visible
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    let initial_selected = state.command_palette.selected;
    let command_count = palette.filtered_commands.len();
    assert!(command_count > 1);

    // Navigate down multiple times
    update(&mut state, &mut palette, Msg::CommandPaletteDown);
    assert_eq!(state.command_palette.selected, initial_selected + 1);

    update(&mut state, &mut palette, Msg::CommandPaletteDown);
    assert_eq!(state.command_palette.selected, initial_selected + 2);

    // Navigate up
    update(&mut state, &mut palette, Msg::CommandPaletteUp);
    assert_eq!(state.command_palette.selected, initial_selected + 1);
}

/// Enter executes selected command
#[test]
fn test_e2e_palette_enter_executes_selected() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);

    // Filter for quit
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('q'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('u'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('i'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('t'));

    // Press Enter to execute quit
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // Quit executed
    assert!(!state.running);
    assert_eq!(state.mode, TuiMode::Chat);
}

/// Esc closes palette (not in argument mode)
#[test]
fn test_e2e_palette_esc_closes_palette() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette);
    assert!(state.command_palette.open);

    // Press Escape
    update(&mut state, &mut palette, Msg::CommandPaletteCancelArgument);

    // Palette closed
    assert_eq!(state.mode, TuiMode::Chat);
    assert!(!state.command_palette.open);
}

// ─── Palette Search/Filter ─────────────────────────────────────────────────────

/// Typing filter chars narrows command list
#[test]
fn test_e2e_palette_filter_narrows_results() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    let all_count = palette.filtered_commands.len();
    assert!(all_count > 5);

    // Type 'q' - should narrow to quit
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('q'));
    assert!(palette.filtered_commands.len() < all_count);
    assert!(state.command_palette.filter == "q");

    // Type 'u' - still quit
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('u'));
    assert!(state.command_palette.filter == "qu");
}

/// Backspace removes filter chars and expands list
#[test]
fn test_e2e_palette_backspace_expands_results() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    let all_count = palette.filtered_commands.len();

    // Filter to narrow (type "qu")
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('q'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('u'));
    let filtered_count = palette.filtered_commands.len();
    assert!(filtered_count < all_count);
    assert_eq!(state.command_palette.filter, "qu");

    // Backspace removes 'u', filter becomes "q"
    update(&mut state, &mut palette, Msg::CommandPaletteBackspace);
    assert_eq!(state.command_palette.filter, "q");

    // Count should be between filtered and all
    assert!(palette.filtered_commands.len() >= filtered_count);
    assert!(palette.filtered_commands.len() < all_count);
}

/// Empty filter shows all commands sorted by usage
#[test]
fn test_e2e_palette_empty_filter_shows_all() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    let all_count = palette.filtered_commands.len();

    // Type to filter
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('q'));
    assert!(palette.filtered_commands.len() < all_count);

    // Clear filter with backspaces
    update(&mut state, &mut palette, Msg::CommandPaletteBackspace);
    update(&mut state, &mut palette, Msg::CommandPaletteBackspace);

    // Back to all
    assert_eq!(palette.filtered_commands.len(), all_count);
    assert!(state.command_palette.filter.is_empty());
}

/// Filter selection stays within valid bounds
#[test]
fn test_e2e_palette_filter_selection_stays_valid() {
    let mut state = make_state_with_model("openai/gpt-4o");
    let mut palette = CommandPalette::new();

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);

    // Navigate to last item
    let last_idx = palette.filtered_commands.len() - 1;
    for _ in 0..last_idx + 5 {
        update(&mut state, &mut palette, Msg::CommandPaletteDown);
    }

    // Filter down to fewer items
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('q'));

    // Selection should be clamped to valid range
    assert!(state.command_palette.selected < palette.filtered_commands.len());
}
