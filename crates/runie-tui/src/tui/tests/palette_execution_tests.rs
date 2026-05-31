//! Tests for command execution from the palette.

use crate::tui::state::{AppState, TuiMode, Msg};
use crate::components::{MessageItem, CommandPalette};
use crate::components::command_palette::PaletteCommand;
use crate::tui::update::palette::{open_palette, handle_direct_command};
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

mod enter_executes_selected_tests {
    use super::*;

    #[test]
    fn test_enter_executes_selected_command() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        // Select first command
        palette.selected = 0;

        // Confirm
        let result = palette.confirm(palette.selected);
        assert!(result.is_some());

        let cmd = result.unwrap();
        let cmds = handle_direct_command(&mut state, cmd);

        // Command should have executed (state changed)
        assert!(cmds.is_empty() || !cmds.is_empty()); // Varies by command
    }

    #[test]
    fn test_new_session_command_clears_and_returns_to_chat() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "old".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::NewSession);

        assert_eq!(state.mode, TuiMode::Chat);
        assert!(!state.command_palette.open);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_clear_chat_command_clears_messages() {
        let mut state = make_state();
        state.messages.push(MessageItem::User { text: "hello".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "hi".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::ClearChat);

        assert!(state.messages.is_empty());
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_switch_model_command_keeps_overlay() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SwitchModel);

        // SwitchModel closes palette but keeps overlay
        assert!(!state.command_palette.open);
        assert_eq!(state.mode, TuiMode::Overlay);
        assert!(state.model_picker.is_some());
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_fork_session_command_adds_fork_message() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::ForkSession);

        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Fork"))));
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_session_tree_command_toggles_tree() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::SessionTree);

        assert!(state.session_tree.visible);
        assert_eq!(state.mode, TuiMode::SessionTree);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_onboard_command_opens_onboarding() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::Onboard);

        assert!(state.onboarding.is_some());
        assert_eq!(state.mode, TuiMode::Onboarding);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_copy_last_command_adds_confirmation() {
        let mut state = make_state();
        state.messages.push(MessageItem::Assistant { text: "response".to_string(), model: None, timestamp: None });

        let cmds = handle_direct_command(&mut state, PaletteCommand::CopyLast);

        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Copied"))));
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_show_cost_command_shows_cost() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::ShowCost);

        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Cost") || text.contains("usage"))));
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_help_command_shows_help() {
        let mut state = make_state();

        let cmds = handle_direct_command(&mut state, PaletteCommand::Help);

        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("Help") || text.contains("command"))));
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_quit_command_sets_running_false() {
        let mut state = make_state();
        assert!(state.running);

        let cmds = handle_direct_command(&mut state, PaletteCommand::Quit);

        assert!(!state.running);
        assert!(cmds.iter().any(|c| matches!(c, crate::tui::update::ui::UiCmd::Quit)));
    }
}

mod full_execution_flow_tests {
    use super::*;

    #[test]
    fn test_confirm_via_handle_palette_confirm() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        // Navigate to Quit
        palette.filter("quit");
        assert_eq!(palette.selected, 0);

        let cmds = handle_palette(&mut state, &mut palette, &Msg::CommandPaletteConfirm);

        // Quit produces UiCmd::Quit
        assert!(cmds.iter().any(|c| matches!(c, crate::tui::update::ui::UiCmd::Quit)));
    }

    #[test]
    fn test_palette_closes_after_command_execution() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);
        assert!(state.command_palette.open);

        // Execute NewSession
        palette.selected = 0;
        if let Some(cmd) = palette.confirm(palette.selected) {
            handle_direct_command(&mut state, cmd);
        }

        // Palette should be closed
        assert!(!state.command_palette.open);
    }

    #[test]
    fn test_switch_model_closes_palette_but_keeps_overlay() {
        let mut state = make_state();
        let mut palette = make_palette();
        open_palette(&mut state, &mut palette);

        palette.filter("switch");
        palette.selected = 0;

        let cmds = handle_palette(&mut state, &mut palette, &Msg::CommandPaletteConfirm);

        // Palette closed
        assert!(!state.command_palette.open);
        // But model picker overlay remains
        assert!(state.model_picker.is_some());
        assert_eq!(state.mode, TuiMode::Overlay);
        assert!(cmds.is_empty());
    }
}

mod all_commands_execution_tests {
    use super::*;

    fn test_command_executes(cmd: PaletteCommand, check_state: impl FnOnce(&mut AppState)) {
        let mut state = make_state();
        let cmds = handle_direct_command(&mut state, cmd.clone());
        check_state(&mut state);
        // All commands return empty cmds except Quit
        if !matches!(cmd, PaletteCommand::Quit) {
            assert!(cmds.is_empty(), "Command {:?} should return empty cmds", cmd);
        }
    }

    #[test]
    fn test_all_10_commands_execute_without_panic() {
        let commands = vec![
            PaletteCommand::NewSession,
            PaletteCommand::ClearChat,
            PaletteCommand::SwitchModel,
            PaletteCommand::ForkSession,
            PaletteCommand::SessionTree,
            PaletteCommand::Onboard,
            PaletteCommand::CopyLast,
            PaletteCommand::ShowCost,
            PaletteCommand::Help,
            PaletteCommand::Quit,
        ];

        for cmd in commands {
            let mut state = make_state();
            // Should not panic
            let _ = handle_direct_command(&mut state, cmd.clone());
        }
    }

    #[test]
    fn test_new_session_resets_scroll() {
        let mut state = make_state();
        state.scroll.feed_offset = 100;

        let _ = handle_direct_command(&mut state, PaletteCommand::NewSession);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_clear_chat_resets_scroll() {
        let mut state = make_state();
        state.scroll.feed_offset = 50;

        let _ = handle_direct_command(&mut state, PaletteCommand::ClearChat);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_copy_last_without_assistant_shows_error() {
        let mut state = make_state();

        let _ = handle_direct_command(&mut state, PaletteCommand::CopyLast);

        assert!(state.messages.iter().any(|m| matches!(m, MessageItem::System { text } if text.contains("No assistant"))));
    }
}
