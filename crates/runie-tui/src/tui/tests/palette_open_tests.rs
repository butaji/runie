//! Tests for opening the command palette.

use crate::tui::state::{AppState, TuiMode};
use crate::components::CommandPalette;
use crate::tui::update::palette::open_palette;

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

mod open_palette_tests {
    use super::*;

    #[test]
    fn test_ctrl_k_opens_palette() {
        let mut state = make_state();
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);

        assert!(state.command_palette.open);
        assert_eq!(state.mode, TuiMode::CommandPalette);
    }

    #[test]
    fn test_open_resets_filter() {
        let mut state = make_state();
        let mut palette = make_palette();

        // Set a filter before opening
        state.command_palette.filter = "test filter".to_string();
        palette.filter("test filter");

        open_palette(&mut state, &mut palette);

        assert!(state.command_palette.filter.is_empty());
        palette.filter("");
        assert_eq!(palette.filtered_commands.len(), 10); // All 10 commands
    }

    #[test]
    fn test_open_resets_selection() {
        let mut state = make_state();
        let mut palette = make_palette();

        // Set selection to non-zero before opening
        state.command_palette.selected = 5;
        palette.selected = 5;

        open_palette(&mut state, &mut palette);

        assert_eq!(state.command_palette.selected, 0);
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn test_open_shows_all_10_commands() {
        let mut state = make_state();
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);

        // There are exactly 10 commands
        assert_eq!(palette.all_commands().len(), 10);
        assert_eq!(palette.filtered_commands.len(), 10);
    }

    #[test]
    fn test_open_resets_argument_mode() {
        let mut state = make_state();
        let mut palette = make_palette();

        // Enter argument mode before opening
        palette.is_argument_mode = true;
        palette.argument_input = "test_input".to_string();
        palette.pending_command = Some("new_session".to_string());

        open_palette(&mut state, &mut palette);

        assert!(!palette.is_argument_mode);
        assert!(palette.argument_input.is_empty());
        assert!(palette.pending_command.is_none());
    }

    #[test]
    fn test_open_from_chat_mode() {
        let mut state = make_state();
        state.mode = TuiMode::Chat;
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);

        assert_eq!(state.mode, TuiMode::CommandPalette);
        assert!(state.command_palette.open);
    }

    #[test]
    fn test_open_from_overlay_mode() {
        let mut state = make_state();
        state.mode = TuiMode::Overlay;
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);

        assert_eq!(state.mode, TuiMode::CommandPalette);
    }

    #[test]
    fn test_open_while_already_open_resets_state() {
        let mut state = make_state();
        let mut palette = make_palette();

        // First open
        open_palette(&mut state, &mut palette);
        palette.filter("quit");
        palette.selected = 3;

        // Open again - should reset
        open_palette(&mut state, &mut palette);

        assert_eq!(state.command_palette.filter.len(), 0);
        assert_eq!(palette.selected, 0);
        assert_eq!(palette.filtered_commands.len(), 10);
    }
}
