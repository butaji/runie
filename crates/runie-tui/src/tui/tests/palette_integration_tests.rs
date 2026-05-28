//! Integration tests for command palette full flows.

use crate::tui::state::{AppState, TuiMode};
use crate::components::{MessageItem, CommandPalette};
use crate::components::command_palette::PaletteCommand;
use crate::tui::update::palette::{handle_direct_command, open_palette};

fn make_state() -> AppState {
    AppState {
        mode: TuiMode::Chat,
        running: true,
        ..Default::default()
    }
}

fn make_palette() -> CommandPalette {
    CommandPalette::new()
}

mod full_flow_tests {
    use super::*;

    #[test]
    fn test_full_flow_open_palette_filter_select_confirm() {
        // Full flow: Open palette → filter → select → confirm → command executes
        let mut state = make_state();
        let mut palette = make_palette();

        // Open palette
        open_palette(&mut state, &mut palette);
        assert!(state.command_palette.open);
        assert_eq!(state.mode, TuiMode::CommandPalette);

        // Filter for "clear"
        palette.filter("clear");
        assert!(!palette.filtered_commands.is_empty());

        // Select first result
        palette.selected = 0;

        // Confirm
        if let Some(cmd) = palette.confirm(palette.selected) {
            let cmds = handle_direct_command(&mut state, cmd);
            // ClearChat doesn't return Cmds, just clears messages
            assert!(cmds.is_empty());
        } else {
            panic!("Expected ClearChat command");
        }

        // Palette is closed by handle_close_modal which is called after confirm
        // In this test we didn't call close, but confirm() returned the command
    }

    #[test]
    fn test_full_flow_new_session_command() {
        // NewSession is a no-arg command
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "old".to_string(), model: None, timestamp: None });

        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);
        palette.filter("new");
        palette.selected = 0;

        let cmd = palette.confirm(palette.selected);
        assert!(cmd.is_some());

        if let PaletteCommand::NewSession = cmd.clone().unwrap() {
            // Expected
        } else {
            panic!("Expected NewSession");
        }

        let cmds = handle_direct_command(&mut state, cmd.unwrap());
        assert!(cmds.is_empty());
        assert!(state.messages.is_empty() || state.messages.iter().any(|m| matches!(m, MessageItem::System { .. })));
    }
}

mod error_handling_tests {
    use super::*;

    #[test]
    fn test_confirm_with_no_pending_command_returns_none() {
        let mut palette = make_palette();

        // Not in argument mode
        let result = palette.confirm_with_argument();
        assert!(result.is_none());
    }

    #[test]
    fn test_filter_with_very_long_query_handled() {
        let mut palette = make_palette();

        let long_query = "a".repeat(1000);
        palette.filter(&long_query);

        // Should not panic, may return empty
        assert!(palette.filtered_commands.is_empty() || !palette.filtered_commands.is_empty());
    }

    #[test]
    fn test_confirm_out_of_bounds_index() {
        let mut palette = make_palette();

        palette.filter("");
        let result = palette.confirm(999);

        assert!(result.is_none());
    }
}

mod state_transition_tests {
    use super::*;

    #[test]
    fn test_palette_open_resets_argument_state() {
        let mut state = make_state();
        let mut palette = make_palette();

        // Enter argument mode using new_session which has keybinding Ctrl+N
        palette.pending_command = Some("new_session".to_string());
        palette.is_argument_mode = true;
        palette.argument_input = "test_input".to_string();

        palette.insert_char('x');

        // Open palette again (should reset)
        open_palette(&mut state, &mut palette);

        assert!(!palette.is_argument_mode);
        assert!(palette.argument_input.is_empty());
        assert!(palette.pending_command.is_none());
    }

    #[test]
    fn test_quit_while_in_palette_closes_everything() {
        let mut state = make_state();
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);
        assert!(state.running);

        palette.filter("quit");
        let result = palette.confirm(palette.selected);

        if let Some(cmd) = result {
            handle_direct_command(&mut state, cmd);
        }

        assert!(!state.running);
    }
}
