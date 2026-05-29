//! App-wide hotkey tests (global, regardless of mode).

use crossterm::event::{KeyCode, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg};
use crate::tui::update::update;
use crate::components::CommandPalette;
use super::helpers::{simulate_key, make_state_with_modal, make_chat_state_with_input};

#[test]
fn test_esc_closes_modal() {
    // Test in CommandPalette mode - P1-1 FIX: Esc sends CancelArgument instead of CloseModal
    // The actual close/argument-cancel behavior happens in update() via handle_palette_escape
    let msg = simulate_key(KeyCode::Esc, KeyModifiers::NONE, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::CommandPaletteCancelArgument), "Esc in CommandPalette should produce Msg::CommandPaletteCancelArgument");

    // Test in DiffViewer mode
    let msg = simulate_key(KeyCode::Esc, KeyModifiers::NONE, TuiMode::DiffViewer);
    assert_eq!(msg, Some(Msg::CloseModal), "Esc in DiffViewer should produce Msg::CloseModal");

    // Test in SessionTree mode
    let msg = simulate_key(KeyCode::Esc, KeyModifiers::NONE, TuiMode::SessionTree);
    assert_eq!(msg, Some(Msg::CloseModal), "Esc in SessionTree should produce Msg::CloseModal");

    // Verify state update - P1-1 FIX: CommandPaletteCancelArgument handles escape properly
    let mut state = make_state_with_modal(TuiMode::CommandPalette);
    let mut palette = CommandPalette::new();
    state.command_palette.open = true;
    update(&mut state, &mut palette, Msg::CommandPaletteCancelArgument);
    // When not in argument mode, CancelArgument closes the palette
    assert!(!state.command_palette.open, "CommandPaletteCancelArgument should close command palette when not in argument mode");
    assert_eq!(state.mode, TuiMode::Chat, "Mode should return to Chat");
}

#[test]
fn test_enter_submits_in_chat() {
    let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::Submit), "Enter in Chat mode should produce Msg::Submit");

    // Verify state update - empty input should not submit
    let mut state = AppState {
        mode: TuiMode::Chat,
        ..Default::default()
    };
    let mut palette = CommandPalette::new();
    let cmds = update(&mut state, &mut palette, Msg::Submit);
    assert!(cmds.is_empty(), "Submit with empty input should produce no commands");
    assert_eq!(state.messages.len(), 0, "No message should be added");

    // Verify state update - non-empty input should submit
    let mut state = make_chat_state_with_input("hello");
    let mut palette = CommandPalette::new();
    let cmds = update(&mut state, &mut palette, Msg::Submit);
    assert!(!cmds.is_empty(), "Submit with input should produce commands");
    // User message + placeholder assistant
    assert_eq!(state.messages.len(), 2, "User + placeholder assistant should be added");
    assert!(state.textarea.is_empty(), "Input should be cleared");
}

#[test]
fn test_enter_selects_in_palette() {
    let msg = simulate_key(KeyCode::Enter, KeyModifiers::NONE, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::CommandPaletteConfirm), "Enter in CommandPalette should produce Msg::CommandPaletteConfirm");

    // Verify state update - CommandPaletteConfirm closes the palette when selecting a no-arg command
    let mut state = make_state_with_modal(TuiMode::CommandPalette);
    let mut palette = CommandPalette::new();
    palette.filter(""); // populate filtered_commands
    // Select a command that does NOT require args (e.g., "new_session")
    if let Some(idx) = palette.filtered_commands.iter().position(|&i| palette.all_commands()[i].id == "new_session") {
        palette.selected = idx;
    }
    state.command_palette.open = true;
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);
    // CommandPaletteConfirm closes the palette when command executes immediately
    assert!(!state.command_palette.open, "CommandPaletteConfirm should close palette for no-arg commands");
}

#[test]
fn test_up_down_navigate_palette() {
    // Up navigation
    let msg = simulate_key(KeyCode::Up, KeyModifiers::NONE, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::CommandPaletteUp), "Up in CommandPalette should produce Msg::CommandPaletteUp");

    // Down navigation
    let msg = simulate_key(KeyCode::Down, KeyModifiers::NONE, TuiMode::CommandPalette);
    assert_eq!(msg, Some(Msg::CommandPaletteDown), "Down in CommandPalette should produce Msg::CommandPaletteDown");

    // Verify state updates - CommandPaletteUp/Down modify selection
    let mut state = make_state_with_modal(TuiMode::CommandPalette);
    let mut palette = CommandPalette::new();
    palette.filter("");
    state.command_palette.selected = 3;
    palette.selected = 3;

    update(&mut state, &mut palette, Msg::CommandPaletteUp);
    assert_eq!(palette.selected, 2, "CommandPaletteUp should decrement selection");
    assert_eq!(state.command_palette.selected, 2, "state selection should be synced");

    update(&mut state, &mut palette, Msg::CommandPaletteDown);
    assert_eq!(palette.selected, 3, "CommandPaletteDown should increment selection");
    assert_eq!(state.command_palette.selected, 3, "state selection should be synced");

    // Test saturation at boundary
    update(&mut state, &mut palette, Msg::CommandPaletteUp);
    update(&mut state, &mut palette, Msg::CommandPaletteUp);
    assert_eq!(palette.selected, 1, "CommandPaletteUp should continue decrementing");
}

#[test]
fn test_page_up_down_scrolls() {
    use crate::components::MessageItem;

    // PageUp
    let msg = simulate_key(KeyCode::PageUp, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ScrollPageUp), "PageUp should produce Msg::ScrollPageUp");

    // PageDown
    let msg = simulate_key(KeyCode::PageDown, KeyModifiers::NONE, TuiMode::Chat);
    assert_eq!(msg, Some(Msg::ScrollPageDown), "PageDown should produce Msg::ScrollPageDown");

    // Verify state updates
    let mut state = AppState {
        mode: TuiMode::Chat,
        messages: (0..20).map(|i| MessageItem::User {
            text: format!("message {}", i),
            model: Some("You".to_string()),
            timestamp: None,
        }).collect(),
        scroll: crate::tui::state::ScrollState::default(),
        ..Default::default()
    };
    let mut palette = CommandPalette::new();

    // Scroll down
    update(&mut state, &mut palette, Msg::ScrollPageDown);
    assert!(state.scroll.feed_offset > 0, "ScrollPageDown should increase offset");

    let offset_after_down = state.scroll.feed_offset;

    // Scroll up
    update(&mut state, &mut palette, Msg::ScrollPageUp);
    assert!(state.scroll.feed_offset < offset_after_down, "ScrollPageUp should decrease offset");

    // Test saturation at boundaries
    state.scroll.feed_offset = 0;
    update(&mut state, &mut palette, Msg::ScrollPageUp);
    assert_eq!(state.scroll.feed_offset, 0, "Scroll should not go below 0");
}

// ─── Command Palette Execution Tests ─────────────────────────────────────────

#[test]
fn test_command_palette_confirm_executes_command() {
    // Test that CommandPaletteConfirm with a selected command executes it
    use crate::tui::update::palette::handle_direct_command;
    

    let mut state = make_state_with_modal(TuiMode::CommandPalette);
    let mut palette = CommandPalette::new();
    state.command_palette.open = true;

    // Filter to "quit" command and select it
    palette.filter("quit");
    assert!(!palette.filtered_commands.is_empty());

    // Get the selected command and confirm it
    let selected_idx = palette.selected;
    if let Some(cmd) = palette.confirm(selected_idx) {
        let cmds = handle_direct_command(&mut state, cmd);
        assert!(!state.running, "Quit command should set running=false");
        assert!(cmds.is_empty()); // Quit doesn't return Cmds
    } else {
        panic!("Expected command to be confirmed");
    }
}

#[test]
fn test_palette_closes_after_command_execution() {
    use crate::tui::update::update;
    use crate::tui::state::Msg;

    let mut state = make_state_with_modal(TuiMode::CommandPalette);
    let mut palette = CommandPalette::new();
    state.command_palette.open = true;

    // Select quit command
    palette.filter("quit");
    palette.selected = 0;

    // Simulate Enter key
    update(&mut state, &mut palette, Msg::CommandPaletteConfirm);

    // Palette should be closed
    assert!(!state.command_palette.open, "Palette should close after command execution");
    assert_eq!(state.mode, TuiMode::Chat, "Mode should return to Chat");
}

#[test]
fn test_palette_escape_not_in_argument_mode_closes_palette() {
    use crate::tui::update::update;
    use crate::tui::state::Msg;

    let mut state = make_state_with_modal(TuiMode::CommandPalette);
    let mut palette = CommandPalette::new();
    state.command_palette.open = true;

    // Press Escape without being in argument mode
    update(&mut state, &mut palette, Msg::CommandPaletteCancelArgument);

    // Palette should close
    assert!(!state.command_palette.open, "Palette should close on Escape");
    assert_eq!(state.mode, TuiMode::Chat, "Mode should return to Chat");
}
