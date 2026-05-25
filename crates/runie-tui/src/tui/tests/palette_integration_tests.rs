//! Integration tests for command palette full flows.

use crate::tui::state::{AppState, TuiMode, Msg, Cmd};
use crate::components::{MessageItem, CommandPalette};
use crate::components::command_palette::PaletteCommand;
use crate::tui::update::update;
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
    fn test_full_flow_open_palette_select_arg_command_enter_argument_confirm() {
        // Full flow: Open palette → select arg command → enter argument → confirm
        let mut state = make_state();
        let mut palette = make_palette();

        // Open palette
        open_palette(&mut state, &mut palette);

        // Select "read" which requires args
        palette.filter("read");
        assert!(!palette.filtered_commands.is_empty());
        palette.selected = 0;

        // Confirm enters argument mode
        let result = palette.confirm(palette.selected);
        assert!(result.is_none(), "Should enter argument mode, not return command");
        assert!(palette.is_argument_mode);

        // Type argument
        for c in "/path/to/file.rs".chars() {
            palette.insert_char(c);
        }
        assert_eq!(palette.argument_input, "/path/to/file.rs");

        // Confirm with argument
        let result = palette.confirm_with_argument();
        assert!(result.is_some());

        if let PaletteCommand::ReadFile { path } = result.unwrap() {
            assert_eq!(path, "/path/to/file.rs");
        } else {
            panic!("Expected ReadFile command");
        }

        // Exit argument mode
        assert!(!palette.is_argument_mode);
        assert!(palette.argument_input.is_empty());
    }

    #[test]
    fn test_full_flow_save_session_command() {
        let mut state = make_state();
        let mut palette = make_palette();

        // Open palette and find save session
        open_palette(&mut state, &mut palette);
        palette.filter("save");
        palette.selected = 0;

        // Confirm enters argument mode (save session requires name)
        let result = palette.confirm(palette.selected);
        assert!(result.is_none());
        assert!(palette.is_argument_mode);

        // Type session name
        for c in "my_awesome_session".chars() {
            palette.insert_char(c);
        }

        // Confirm
        let cmd = palette.confirm_with_argument();
        assert!(cmd.is_some());

        if let PaletteCommand::SaveSession { name } = cmd.clone().unwrap() {
            assert_eq!(name, "my_awesome_session");
        } else {
            panic!("Expected SaveSession command");
        }

        // Execute the command
        let cmds = handle_direct_command(&mut state, cmd.unwrap());
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::SaveSession { name: Some("my_awesome_session".to_string()) });
    }

    #[test]
    fn test_full_flow_delete_file_command() {
        let mut state = make_state();
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);
        palette.filter("delete");
        palette.selected = 0;

        let result = palette.confirm(palette.selected);
        assert!(result.is_none());
        assert!(palette.is_argument_mode);

        for c in "/tmp/old_file.txt".chars() {
            palette.insert_char(c);
        }

        let cmd = palette.confirm_with_argument();
        assert!(cmd.is_some());

        if let PaletteCommand::DeleteFile { path } = cmd.clone().unwrap() {
            assert_eq!(path, "/tmp/old_file.txt");
        }

        let cmds = handle_direct_command(&mut state, cmd.unwrap());
        assert_eq!(cmds[0], Cmd::DeleteFile { path: "/tmp/old_file.txt".to_string() });
    }

    #[test]
    fn test_full_flow_compact_context_command() {
        // CompactContext is a no-arg command
        let mut state = make_state();
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);
        palette.filter("compact");
        palette.selected = 0;

        let cmd = palette.confirm(palette.selected);
        assert!(cmd.is_some());

        if let PaletteCommand::CompactContext = cmd.clone().unwrap() {
            // Expected
        } else {
            panic!("Expected CompactContext");
        }

        let cmds = handle_direct_command(&mut state, cmd.unwrap());
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], Cmd::CompactContext);
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

        // Enter argument mode
        palette.filter("load");
        palette.confirm(0);
        assert!(palette.is_argument_mode);

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
