//! Tests for command palette overlap behavior.
//!
//! Ensures that opening/closing the palette properly manages
//! mutual exclusion with other overlays like slash_menu and file_picker.

use crate::tui::state::{AppState, TuiMode};
use crate::components::CommandPalette;
use crate::tui::update::palette::{open_palette, handle_close_modal};

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

mod open_palette_closes_overlays {
    use super::*;

    #[test]
    fn test_palette_closes_slash_menu() {
        let mut state = make_state();
        state.slash_menu.open("/");
        let mut palette = make_palette();

        assert!(state.slash_menu.is_open());

        open_palette(&mut state, &mut palette);

        assert!(!state.slash_menu.is_open(), "slash_menu should be closed when palette opens");
    }

    #[test]
    fn test_palette_closes_file_picker() {
        let mut state = make_state();
        state.file_picker.open();
        let mut palette = make_palette();

        assert!(state.file_picker.is_open());

        open_palette(&mut state, &mut palette);

        assert!(!state.file_picker.is_open(), "file_picker should be closed when palette opens");
    }

    #[test]
    fn test_palette_sets_command_palette_mode() {
        let mut state = make_state();
        let mut palette = make_palette();

        assert_eq!(state.mode, TuiMode::Chat);

        open_palette(&mut state, &mut palette);

        assert_eq!(state.mode, TuiMode::CommandPalette);
    }

    #[test]
    fn test_palette_opens_when_called() {
        let mut state = make_state();
        let mut palette = make_palette();

        assert!(!state.command_palette.open);

        open_palette(&mut state, &mut palette);

        assert!(state.command_palette.open);
    }

    #[test]
    fn test_palette_clears_filter_on_open() {
        let mut state = make_state();
        state.command_palette.filter = "previous filter".to_string();
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);

        assert!(state.command_palette.filter.is_empty());
    }

    #[test]
    fn test_palette_resets_selection_on_open() {
        let mut state = make_state();
        state.command_palette.selected = 99;
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);

        assert_eq!(state.command_palette.selected, 0);
    }

    #[test]
    fn test_palette_disables_argument_mode() {
        let mut state = make_state();
        let mut palette = make_palette();
        palette.is_argument_mode = true;
        palette.argument_input = "some input".to_string();

        open_palette(&mut state, &mut palette);

        assert!(!palette.is_argument_mode);
        assert!(palette.argument_input.is_empty());
    }
}

mod palette_mutual_exclusion {
    use super::*;

    #[test]
    fn test_opening_palette_closes_both_overlays() {
        let mut state = make_state();
        state.slash_menu.open("/");
        state.file_picker.open();
        let mut palette = make_palette();

        assert!(state.slash_menu.is_open());
        assert!(state.file_picker.is_open());

        open_palette(&mut state, &mut palette);

        assert!(!state.slash_menu.is_open());
        assert!(!state.file_picker.is_open());
    }

    #[test]
    fn test_only_palette_overlay_is_active_when_opened() {
        let mut state = make_state();
        state.slash_menu.open("/");
        state.file_picker.open();
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);

        // Mode should be CommandPalette
        assert_eq!(state.mode, TuiMode::CommandPalette);
        // Both overlays should be closed
        assert!(!state.slash_menu.is_open());
        assert!(!state.file_picker.is_open());
        // Palette should be open
        assert!(state.command_palette.open);
    }

    #[test]
    fn test_calling_open_palette_twice_is_idempotent() {
        let mut state = make_state();
        let mut palette = make_palette();

        open_palette(&mut state, &mut palette);
        open_palette(&mut state, &mut palette);

        // Should still be in correct state
        assert_eq!(state.mode, TuiMode::CommandPalette);
        assert!(state.command_palette.open);
        assert!(!state.slash_menu.is_open());
        assert!(!state.file_picker.is_open());
    }
}

mod close_modal_preserves_state {
    use super::*;

    #[test]
    fn test_closing_palette_preserves_slash_menu_state() {
        let mut state = make_state();
        // slash_menu was already closed before palette opened
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        handle_close_modal(&mut state);

        // slash_menu should still be closed (was already closed)
        assert!(!state.slash_menu.is_open());
    }

    #[test]
    fn test_closing_palette_preserves_file_picker_state() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        handle_close_modal(&mut state);

        // file_picker should still be closed (was already closed)
        assert!(!state.file_picker.is_open());
    }

    #[test]
    fn test_closing_palette_resets_mode_to_chat() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        handle_close_modal(&mut state);

        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_closing_palette_closes_palette() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        handle_close_modal(&mut state);

        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_closing_palette_clears_filter() {
        let mut state = make_state();
        state.command_palette.filter = "some filter".to_string();

        handle_close_modal(&mut state);

        assert!(state.command_palette.filter.is_empty());
    }

    #[test]
    fn test_closing_palette_resets_selection() {
        let mut state = make_state();
        state.command_palette.selected = 42;

        handle_close_modal(&mut state);

        assert_eq!(state.command_palette.selected, 0);
    }
}

mod overlay_background_clear {
    // Background clearing is verified by render tests.
    // This module documents the expected behavior:
    //
    // When render_command_palette is called (overlays.rs:86-109):
    // 1. dim_background is called first to dim the area
    // 2. Then the palette area is explicitly cleared by setting
    //    all cells to space character with bg.base color
    // 3. Finally, the palette widget is rendered on top
    //
    // This ensures no visual overlap from previous content.

    #[test]
    fn test_palette_render_documentation() {
        // This test documents the expected rendering behavior.
        // The actual rendering is done in pipe/render/overlays.rs
        // render_command_palette function (lines 86-109).
        //
        // Key implementation details:
        // - Lines 97-106: Clear palette area by iterating over
        //   palette_area and setting each cell to ' ' with bg_base
        // - This happens BEFORE palette.render_ref is called
        // - This prevents overlap with content that was previously
        //   in the same screen area
        assert!(true);
    }
}
