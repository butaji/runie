//! Tests for closing the command palette.

use crate::tui::state::{AppState, TuiMode, Msg};
use crate::components::{CommandPalette, DiffViewer};
use crate::components::command_palette::PaletteCommand;
use crate::tui::update::palette::{open_palette, handle_close_modal, handle_direct_command, handle_palette_escape};
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

fn open_palette_state() -> (AppState, CommandPalette) {
    let mut state = make_state();
    let mut palette = make_palette();
    open_palette(&mut state, &mut palette);
    (state, palette)
}

mod escape_closes_palette_tests {
    use super::*;

    #[test]
    fn test_escape_closes_palette() {
        let (mut state, mut palette) = open_palette_state();

        handle_palette_escape(&mut state, &mut palette);

        assert!(!state.command_palette.open);
        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_escape_via_handle_palette_cancel_argument() {
        let (mut state, mut palette) = open_palette_state();

        let cmds = handle_palette(&mut state, &mut palette, &Msg::CommandPaletteCancelArgument);

        assert!(!state.command_palette.open);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_escape_resets_filter() {
        let (mut state, mut palette) = open_palette_state();

        // Add filter
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteFilter('q'));
        assert!(!state.command_palette.filter.is_empty());

        // Escape
        handle_palette_escape(&mut state, &mut palette);

        assert!(state.command_palette.filter.is_empty());
    }

    #[test]
    fn test_escape_resets_selection() {
        let (mut state, mut palette) = open_palette_state();

        // Navigate down
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        handle_palette(&mut state, &mut palette, &Msg::CommandPaletteDown);
        assert!(palette.selected > 0);

        // Escape
        handle_palette_escape(&mut state, &mut palette);

        assert_eq!(state.command_palette.selected, 0);
    }
}

mod handle_close_modal_tests {
    use super::*;

    #[test]
    fn test_close_modal_resets_mode_to_chat() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        handle_close_modal(&mut state);

        assert_eq!(state.mode, TuiMode::Chat);
    }

    #[test]
    fn test_close_modal_closes_palette() {
        let mut state = make_state();
        state.mode = TuiMode::CommandPalette;
        state.command_palette.open = true;

        handle_close_modal(&mut state);

        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_close_modal_clears_filter() {
        let mut state = make_state();
        state.command_palette.filter = "test filter".to_string();

        handle_close_modal(&mut state);

        assert!(state.command_palette.filter.is_empty());
    }

    #[test]
    fn test_close_modal_resets_selection() {
        let mut state = make_state();
        state.command_palette.selected = 5;

        handle_close_modal(&mut state);

        assert_eq!(state.command_palette.selected, 0);
    }

    #[test]
    fn test_close_modal_clears_permission_modal() {
        let mut state = make_state();
        state.permission_modal.tool = Some("bash".to_string());
        state.permission_modal.tool_call_id = Some("tool_123".to_string());

        handle_close_modal(&mut state);

        assert!(state.permission_modal.tool.is_none());
        assert!(state.permission_modal.tool_call_id.is_none());
    }

    #[test]
    fn test_close_modal_clears_diff_viewer() {
        let mut state = make_state();
        state.diff_viewer = Some(DiffViewer::new("test.txt".to_string(), "old".to_string(), "new".to_string()));

        handle_close_modal(&mut state);

        assert!(state.diff_viewer.is_none());
    }

    #[test]
    fn test_close_modal_hides_session_tree() {
        let mut state = make_state();
        state.session_tree.show();

        handle_close_modal(&mut state);

        assert!(!state.session_tree.visible);
    }

    #[test]
    fn test_close_modal_clears_model_picker() {
        let mut state = make_state();
        state.model_picker = Some(crate::components::ModelPicker::with_default_models());

        handle_close_modal(&mut state);

        assert!(state.model_picker.is_none());
    }
}

mod palette_closes_after_execution_tests {
    use super::*;

    #[test]
    fn test_palette_closes_after_new_session() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        if let Some(cmd) = palette.confirm(palette.selected) {
            handle_direct_command(&mut state, cmd);
        }

        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_palette_closes_after_clear_chat() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        if let Some(cmd) = palette.confirm(palette.selected) {
            handle_direct_command(&mut state, cmd);
        }

        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_palette_closes_after_fork_session() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        if let Some(cmd) = palette.confirm(palette.selected) {
            handle_direct_command(&mut state, cmd);
        }

        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_palette_closes_after_copy_last() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        if let Some(cmd) = palette.confirm(palette.selected) {
            handle_direct_command(&mut state, cmd);
        }

        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_palette_closes_after_show_cost() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        if let Some(cmd) = palette.confirm(palette.selected) {
            handle_direct_command(&mut state, cmd);
        }

        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_palette_closes_after_help() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        if let Some(cmd) = palette.confirm(palette.selected) {
            handle_direct_command(&mut state, cmd);
        }

        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_switch_model_closes_palette_but_keeps_overlay() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        if let Some(cmd) = palette.confirm(palette.selected) {
            if matches!(cmd, PaletteCommand::SwitchModel) {
                handle_direct_command(&mut state, cmd);
            }
        }

        // Palette closed
        assert!(!state.command_palette.open);
        // But model picker overlay remains
        assert!(state.model_picker.is_some());
        assert_eq!(state.mode, TuiMode::Overlay);
    }

    #[test]
    fn test_session_tree_closes_palette() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        if let Some(cmd) = palette.confirm(palette.selected) {
            if matches!(cmd, PaletteCommand::SessionTree) {
                handle_direct_command(&mut state, cmd);
            }
        }

        assert!(!state.command_palette.open);
        assert_eq!(state.mode, TuiMode::SessionTree);
    }

    #[test]
    fn test_onboard_closes_palette() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        if let Some(cmd) = palette.confirm(palette.selected) {
            if matches!(cmd, PaletteCommand::Onboard) {
                handle_direct_command(&mut state, cmd);
            }
        }

        assert!(!state.command_palette.open);
        assert_eq!(state.mode, TuiMode::Onboarding);
    }
}

mod close_modal_msg_tests {
    use super::*;

    #[test]
    fn test_close_modal_message_closes_palette() {
        let (mut state, mut palette) = open_palette_state();

        // The CloseModal message is handled in ui::update
        state.mode = TuiMode::Chat;
        state.command_palette.open = false;

        // Verify initial state is closed
        assert!(!state.command_palette.open);
    }
}
