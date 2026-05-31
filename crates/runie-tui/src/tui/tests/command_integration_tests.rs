//! Integration tests for command execution flow.
//!
//! Tests verify the complete flow from user input (slash commands and palette commands)
//! through to command execution and state changes.

use crate::tui::state::{AppState, Msg, TuiMode, CommandPaletteState, ScrollState, ContextState, PermissionModalState, AnimationState, ClearInputConfirm, TopBarState};
use crate::components::{CommandPalette, MessageItem};
use crate::tui::update::update;
use runie_ai::TokenUsage as AiTokenUsage;
use crate::components::SessionTreeNavigator;
use ratatui_textarea::TextArea;

// ─── Test Helpers ───────────────────────────────────────────────────────────────

pub fn make_state() -> AppState {
    AppState {
        messages: Vec::new(),
        textarea: TextArea::default(),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: Some("openai/gpt-4o".to_string()),
        context: ContextState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: SessionTreeNavigator::new(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: ClearInputConfirm::default(),
        model_picker: None,
        agent_start_time: None,
        input_history: Vec::new(),
        input_history_index: None,
        input_draft: String::new(),
        status_header: None,
        status_details: None,
        status_start_time: None,
        thinking_start: None,
        thinking_duration: None,
        is_thinking: false,
        mock_mode: false,
        top_bar: TopBarState::default(),
    }
}

// ─── Test 1: Slash /clear command ───────────────────────────────────────────────

#[test]
fn test_slash_clear_command_clears_messages() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Add some messages
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "Hi there".to_string(), model: None, timestamp: None });

    // User types "/clear" in chat input → SlashCommand dispatched
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Clear));

    // Messages should be cleared
    assert!(state.messages.is_empty(), "Messages should be cleared by /clear");
    // Mode should stay in Chat
    assert_eq!(state.mode, TuiMode::Chat, "Mode should stay Chat after /clear");
}

// ─── Test 2: Slash /new command ─────────────────────────────────────────────────

#[test]
fn test_slash_new_command_clears_and_starts_new_session() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Add some messages
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "Hi there".to_string(), model: None, timestamp: None });

    // User types "/new" in chat input → SlashCommand dispatched
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::New));

    // Original messages should be cleared (only system message remains)
    assert_eq!(state.messages.len(), 1, "Should have exactly one message (the system message)");
    // New session system message should be added
    assert!(
        matches!(&state.messages[0], MessageItem::System { text } if text.contains("New session")),
        "Should have 'New session started' system message"
    );
    // Scroll offset should reset
    assert_eq!(state.scroll.feed_offset, 0, "Scroll offset should reset");
    // Mode should stay in Chat
    assert_eq!(state.mode, TuiMode::Chat, "Mode should stay Chat after /new");
}

// ─── Test 3: Slash /help command ───────────────────────────────────────────────

#[test]
fn test_slash_help_command_shows_help() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // User types "/help" in chat input → SlashCommand dispatched
    update(&mut state, &mut palette, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Help));

    // Help system message should be added
    assert_eq!(state.messages.len(), 1, "Should have exactly one message");
    assert!(
        matches!(&state.messages[0], MessageItem::System { text } if text.contains('/')),
        "Help message should contain '/' (listing commands)"
    );
    // Mode should stay in Chat
    assert_eq!(state.mode, TuiMode::Chat, "Mode should stay Chat after /help");
}

// ─── Test 4: Palette Clear Chat command ────────────────────────────────────────

#[test]
fn test_palette_clear_chat_command_clears_messages() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Add some messages
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "Hi there".to_string(), model: None, timestamp: None });

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert_eq!(state.mode, TuiMode::CommandPalette, "Should be in CommandPalette mode");

    // Filter for "clear"
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('c'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('l'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('e'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('a'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('r'));

    // Confirm - should select "Clear Chat"
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // Messages should be cleared
    assert!(state.messages.is_empty(), "Messages should be cleared by palette Clear Chat");
    // Palette should be closed (mode back to Chat)
    assert_eq!(state.mode, TuiMode::Chat, "Palette should close after Clear Chat");
}

// ─── Test 5: Both paths produce same result ─────────────────────────────────────

#[test]
fn test_slash_clear_and_palette_clear_produce_same_result() {
    // Test state 1: /clear slash command
    let mut state1 = make_state();
    let mut palette1 = CommandPalette::new();
    state1.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state1.messages.push(MessageItem::Assistant { text: "Hi there".to_string(), model: None, timestamp: None });
    state1.scroll.feed_offset = 5;

    update(&mut state1, &mut palette1, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Clear));

    // Test state 2: Clear Chat via palette
    let mut state2 = make_state();
    let mut palette2 = CommandPalette::new();
    state2.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state2.messages.push(MessageItem::Assistant { text: "Hi there".to_string(), model: None, timestamp: None });
    state2.scroll.feed_offset = 5;

    // Open palette, filter "clear", confirm
    update(&mut state2, &mut palette2, Msg::OpenCommandPalette);
    update(&mut state2, &mut palette2, Msg::CommandPaletteFilter('c'));
    update(&mut state2, &mut palette2, Msg::CommandPaletteFilter('l'));
    update(&mut state2, &mut palette2, Msg::CommandPaletteFilter('e'));
    update(&mut state2, &mut palette2, Msg::CommandPaletteFilter('a'));
    update(&mut state2, &mut palette2, Msg::CommandPaletteFilter('r'));
    update(&mut state2, &mut palette2, Msg::CommandPaletteConfirm);

    // Both should have cleared messages
    assert_eq!(state1.messages.len(), state2.messages.len(), "Both should have cleared messages");
    assert!(state1.messages.is_empty() && state2.messages.is_empty(), "Both should have empty messages");

    // Both should have reset scroll offset
    assert_eq!(state1.scroll.feed_offset, state2.scroll.feed_offset, "Both should have same scroll offset");
    assert_eq!(state1.scroll.feed_offset, 0, "Scroll offset should be 0");

    // Both should be in Chat mode
    assert_eq!(state1.mode, state2.mode, "Both should be in same mode");
    assert_eq!(state1.mode, TuiMode::Chat, "Both should be in Chat mode");
}

// ─── Additional verification tests ─────────────────────────────────────────────

#[test]
fn test_direct_command_clear_chat_is_equivalent_to_slash_clear() {
    // Test that Msg::DirectCommand(PaletteCommand::ClearChat) produces same result as slash
    let mut state_slash = make_state();
    let mut palette_slash = CommandPalette::new();
    state_slash.messages.push(MessageItem::User { text: "Test".to_string(), model: None, timestamp: None });

    update(&mut state_slash, &mut palette_slash, Msg::SlashCommand(runie_core::slash_command::SlashCommand::Clear));

    let mut state_direct = make_state();
    let mut palette_direct = CommandPalette::new();
    state_direct.messages.push(MessageItem::User { text: "Test".to_string(), model: None, timestamp: None });

    update(&mut state_direct, &mut palette_direct, Msg::DirectCommand(crate::components::PaletteCommand::ClearChat));

    assert_eq!(state_slash.messages.len(), state_direct.messages.len());
    assert_eq!(state_slash.mode, state_direct.mode);
}

#[test]
fn test_palette_confirm_flow_full_integration() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Add messages
    state.messages.push(MessageItem::User { text: "Hello".to_string(), model: None, timestamp: None });
    state.messages.push(MessageItem::Assistant { text: "Hi".to_string(), model: None, timestamp: None });

    // Open palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);
    assert!(state.command_palette.open);

    // Filter and confirm "Clear Chat"
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('c'));
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // Verify end state
    assert!(state.messages.is_empty());
    assert_eq!(state.mode, TuiMode::Chat);
    assert!(!state.command_palette.open);
}
