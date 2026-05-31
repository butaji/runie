//! Tests for navigation within the command palette.

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

mod navigation_tests {
    use super::*;

    #[test]
    fn test_down_arrow_increments_selection() {
        let (mut state, mut palette) = open_and_get_palette();

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);

        assert_eq!(palette.selected, 1);
        assert_eq!(state.command_palette.selected, 1);
    }

    #[test]
    fn test_up_arrow_decrements_selection() {
        let (mut state, mut palette) = open_and_get_palette();

        // First go down to position 2
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        assert_eq!(palette.selected, 2);

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteUp);

        assert_eq!(palette.selected, 1);
    }

    #[test]
    fn test_up_at_zero_stays_at_zero() {
        let (mut state, mut palette) = open_and_get_palette();

        // Already at 0
        assert_eq!(palette.selected, 0);

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteUp);

        assert_eq!(palette.selected, 0);
        assert_eq!(state.command_palette.selected, 0);
    }

    #[test]
    fn test_down_at_max_stays_at_max() {
        let (mut state, mut palette) = open_and_get_palette();

        // Navigate to last item (index 9 for 10 items)
        for _ in 0..20 {
            handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        }

        let max_index = palette.filtered_commands.len() - 1;
        assert_eq!(palette.selected, max_index);
    }

    #[test]
    fn test_navigation_respects_filtered_list() {
        let (mut state, mut palette) = open_and_get_palette();

        // Filter to only 2 items
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('q'));
        // "quit" shows only Quit command
        let max = palette.filtered_commands.len() - 1; // Should be 0

        // Navigate down many times
        for _ in 0..10 {
            handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        }

        assert_eq!(palette.selected, max);
    }

    #[test]
    fn test_multiple_down_cycles() {
        let (mut state, mut palette) = open_and_get_palette();

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteUp);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteUp);

        assert_eq!(palette.selected, 1);
    }

    #[test]
    fn test_navigation_updates_state同步() {
        let (mut state, mut palette) = open_and_get_palette();

        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);

        // State and palette selection should be in sync
        assert_eq!(state.command_palette.selected, palette.selected);
    }

    #[test]
    fn test_down_from_middle_to_max() {
        let (mut state, mut palette) = open_and_get_palette();

        // Go to middle
        for _ in 0..5 {
            handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        }
        assert_eq!(palette.selected, 5);

        // Continue to max
        let max = palette.filtered_commands.len() - 1;
        for _ in 0..10 {
            handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        }
        assert_eq!(palette.selected, max);
    }

    #[test]
    fn test_up_from_middle_to_zero() {
        let (mut state, mut palette) = open_and_get_palette();

        // Go to middle
        for _ in 0..5 {
            handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        }
        assert_eq!(palette.selected, 5);

        // Continue up to zero
        for _ in 0..10 {
            handle_palette(&mut state, &mut palette, &Msg::CommandPaletteUp);
        }
        assert_eq!(palette.selected, 0);
    }
}
