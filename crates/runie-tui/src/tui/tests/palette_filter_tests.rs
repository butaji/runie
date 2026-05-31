//! Tests for filtering commands in the palette.

use crate::tui::state::{AppState, TuiMode, Msg};
use crate::components::CommandPalette;
use crate::tui::update::palette::open_palette;
use crate::tui::update::ui::palette_helpers::handle_palette;

fn make_state() -> AppState {
    AppState {
        mode: TuiMode::CommandPalette,
        running: true,
        ..Default::default()
    }
}

fn make_palette() -> CommandPalette {
    CommandPalette::new()
}

fn open_and_get_palette() -> (AppState, CommandPalette) {
    let mut state = make_state();
    let mut palette = make_palette();
    open_palette(&mut state, &mut palette);
    (state, palette)
}

mod filter_tests {
    use super::*;

    #[test]
    fn test_filter_narrows_commands() {
        let (mut state, mut palette) = open_and_get_palette();

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('c'));

        // Should match "Clear Chat" and "Copy Last Response"
        assert!(palette.filtered_commands.len() < 10);
        assert!(!palette.filtered_commands.is_empty());
    }

    #[test]
    fn test_filter_to_zero_items() {
        let (mut state, mut palette) = open_and_get_palette();

        // No command matches "xyz123"
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('x'));
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('y'));
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('z'));

        assert!(palette.filtered_commands.is_empty());
    }

    #[test]
    fn test_filter_to_one_item() {
        let (mut state, mut palette) = open_and_get_palette();

        // "quit" matches only Quit
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('q'));
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('u'));
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('i'));
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('t'));

        assert_eq!(palette.filtered_commands.len(), 1);
    }

    #[test]
    fn test_empty_filter_shows_all() {
        let (mut state, mut palette) = open_and_get_palette();

        // Filter to narrow, then backspace all
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('c'));
        assert!(palette.filtered_commands.len() < 10);

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteBackspace);
        // After removing 'c', empty filter shows all
        assert_eq!(palette.filtered_commands.len(), 10);
    }

    #[test]
    fn test_filter_resets_selection_to_zero() {
        let (mut state, mut palette) = open_and_get_palette();

        // Navigate down a bit
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        assert_eq!(palette.selected, 2);

        // Now filter - selection should reset to 0
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('q'));

        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn test_backspace_removes_filter_chars() {
        let (mut state, mut palette) = open_and_get_palette();

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('c'));
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('l'));
        assert_eq!(state.command_palette.filter.len(), 2);

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteBackspace);
        assert_eq!(state.command_palette.filter.len(), 1);

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteBackspace);
        assert_eq!(state.command_palette.filter.len(), 0);
    }

    #[test]
    fn test_backspace_on_empty_filter_does_not_panic() {
        let (mut state, mut palette) = open_and_get_palette();

        // Should not panic
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteBackspace);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteBackspace);

        assert_eq!(state.command_palette.filter.len(), 0);
    }

    #[test]
    fn test_filter_resets_to_zero_when_results_shrink() {
        let (mut state, mut palette) = open_and_get_palette();

        // Navigate to index 5
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        assert_eq!(palette.selected, 5);

        // Filter to only 2 items - selection should cap to 1
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('q'));
        assert!(palette.selected <= palette.filtered_commands.len().saturating_sub(1));
    }

    #[test]
    fn test_filter_alias_matching() {
        let (mut state, mut palette) = open_and_get_palette();

        // "n" alias matches NewSession
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('n'));

        assert!(!palette.filtered_commands.is_empty());
        // Should include NewSession
        let idx = palette.filtered_commands[palette.selected];
        let cmd = &palette.all_commands()[idx];
        assert!(cmd.label.contains("New") || cmd.aliases.contains(&"n".to_string()));
    }

    #[test]
    fn test_filter_case_insensitive() {
        let (mut state, mut palette) = open_and_get_palette();

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('Q'));
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('U'));
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('I'));
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('T'));

        // Should still match Quit
        assert_eq!(palette.filtered_commands.len(), 1);
    }
}
