//! ReplyProvider input history and command palette tests.
//!
//! Tests input history navigation and command palette behavior
//! using ReplyProvider for agent simulation.

use crate::components::CommandPalette;
use crate::tui::state::{AppState, Msg, TuiMode};
use crate::tui::update::update;
use ratatui_textarea::TextArea;
use runie_ai::TokenUsage as AiTokenUsage;
use crate::components::MessageItem;

/// Helper: Create AppState with default values.
fn make_state() -> AppState {
    AppState {
        messages: vec![],
        textarea: TextArea::default(),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: Some("MiniMax-M2.7-highspeed".to_string()),
        context: Default::default(),
        permission_modal: Default::default(),
        command_palette: Default::default(),
        scroll: Default::default(),
        animation: Default::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: Default::default(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: Default::default(),
        model_picker: None,
        agent_start_time: None,
        input_history: Vec::new(),
        input_history_index: None,
        input_draft: String::new(),
        status_header: None,
        status_details: None,
        status_start_time: None,
        thinking: None,
        mock_mode: false,
        top_bar: Default::default(),
        last_turn_duration_secs: None,
        last_turn_tokens: None,
        last_turn_tool_calls: None,
        show_thoughts: false,
    }
}

/// Helper: Create state with input history pre-populated.
fn make_state_with_history(history: Vec<&str>) -> AppState {
    let mut state = make_state();
    state.input_history = history.into_iter().map(String::from).collect();
    state
}

/// Helper: Create state with messages for testing command effects.
fn make_state_with_messages() -> AppState {
    let mut state = make_state();
    state.messages.push(MessageItem::User {
        text: "Hello".to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    state.messages.push(MessageItem::Assistant {
        text: "Hi there!".to_string(),
        model: state.current_model.clone(),
        timestamp: None,
    });
    state
}

// ─── Input History Tests ─────────────────────────────────────────────────────

#[test]
fn test_input_history_stores_submitted_message() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Type a message
    state.textarea = TextArea::new(vec!["hello".to_string()]);

    // Submit
    update(&mut state, &mut palette, Msg::Submit);

    // Assert "hello" is in input_history
    assert!(state.input_history.contains(&"hello".to_string()));
}

#[test]
fn test_input_history_navigate_up() {
    let mut state = make_state_with_history(vec!["msg1", "msg2", "msg3"]);
    let mut palette = CommandPalette::new();

    // Press Up arrow
    update(&mut state, &mut palette, Msg::HistoryUp);

    // Assert textarea shows "msg3" (last submitted)
    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "msg3");
}

#[test]
fn test_input_history_navigate_down() {
    let mut state = make_state_with_history(vec!["msg1", "msg2", "msg3"]);
    let mut palette = CommandPalette::new();

    // Navigate up twice to get to "msg2"
    update(&mut state, &mut palette, Msg::HistoryUp);
    update(&mut state, &mut palette, Msg::HistoryUp);
    assert_eq!(state.textarea.lines().join("\n"), "msg2");

    // Press Down arrow
    update(&mut state, &mut palette, Msg::HistoryDown);

    // Assert textarea shows "msg3" (newer message)
    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "msg3");
}

#[test]
fn test_input_history_at_bottom_returns_to_draft() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Set up history - viewing msg3 (newest, index 2)
    state.input_history = vec!["msg1".to_string(), "msg2".to_string(), "msg3".to_string()];
    state.input_draft = "draft text".to_string();
    state.input_history_index = Some(2); // Currently viewing msg3 (newest)
    state.textarea = TextArea::new(vec!["msg3".to_string()]);

    // Navigate down to return to draft
    update(&mut state, &mut palette, Msg::HistoryDown);

    // Assert textarea shows draft text
    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "draft text");
    assert!(state.input_history_index.is_none());
}

#[test]
fn test_input_history_empty_no_change() {
    let mut state = make_state();
    state.textarea = TextArea::new(vec!["some text".to_string()]);
    let mut palette = CommandPalette::new();

    // Press Up with empty history
    update(&mut state, &mut palette, Msg::HistoryUp);

    // Assert textarea unchanged
    let text = state.textarea.lines().join("\n");
    assert_eq!(text, "some text");
}

// ─── Command Palette Tests ───────────────────────────────────────────────────

#[test]
fn test_command_palette_opens() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Open command palette
    update(&mut state, &mut palette, Msg::OpenCommandPalette);

    // Assert mode is CommandPalette and open is true
    assert_eq!(state.mode, TuiMode::CommandPalette);
    assert!(state.command_palette.open);
}

#[test]
fn test_command_palette_closes_with_escape() {
    let mut state = make_state();
    state.mode = TuiMode::CommandPalette;
    state.command_palette.open = true;
    let mut palette = CommandPalette::new();

    // Press Escape (CloseModal)
    update(&mut state, &mut palette, Msg::CloseModal);

    // Assert mode returns to Chat
    assert_eq!(state.mode, TuiMode::Chat);
    assert!(!state.command_palette.open);
}

#[test]
fn test_command_palette_selects_command() {
    let mut state = make_state_with_messages();
    let mut palette = CommandPalette::new();

    // Ensure we have some messages
    assert!(!state.messages.is_empty());

    // Open palette - this populates filtered_commands
    update(&mut state, &mut palette, Msg::OpenCommandPalette);

    // Navigate down to ClearChat (index 1 in the command list)
    update(&mut state, &mut palette, Msg::CommandPaletteDown);
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // After selecting ClearChat, messages should be cleared
    assert!(state.messages.is_empty());
}

#[test]
fn test_command_palette_filters_commands() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Open palette and filter
    update(&mut state, &mut palette, Msg::OpenCommandPalette);

    // Filter by "cle" - should show ClearChat
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('c'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('l'));
    update(&mut state, &mut palette, Msg::CommandPaletteFilter('e'));

    // Only commands matching "cle" should be shown
    let filtered = palette
        .filtered_commands
        .iter()
        .filter_map(|&idx| palette.all_commands().get(idx))
        .collect::<Vec<_>>();

    assert!(filtered.iter().any(|cmd| cmd.id == "clear_chat"));
}

#[test]
fn test_command_palette_model_command() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Open palette - this populates filtered_commands
    update(&mut state, &mut palette, Msg::OpenCommandPalette);

    // Navigate down to SwitchModel (index 4 in the command list)
    // Commands are: new_session(0), clear_chat(1), fork(2), tree(3), switch_model(4), ...
    for _ in 0..4 {
        update(&mut state, &mut palette, Msg::CommandPaletteDown);
    }

    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // After selecting SwitchModel, model_picker should open
    assert!(state.model_picker.is_some());
}
