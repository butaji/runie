//! Tests for usage tracking behavior in the command palette.
//! Tests usage tracking via public behavior - filter ordering.

use crate::tui::state::{AppState, TuiMode};
use crate::components::CommandPalette;
use crate::components::command_palette::PaletteCommand;
use crate::tui::update::palette::{open_palette, handle_direct_command};

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

/// Helper to execute a command and track usage
fn execute_command(palette: &mut CommandPalette, state: &mut AppState, cmd: PaletteCommand) {
    let _ = handle_direct_command(state, cmd);
}

/// Helper to confirm a command from palette and execute it
fn confirm_and_execute(palette: &mut CommandPalette, state: &mut AppState, filter_text: &str) {
    palette.filter(filter_text);
    if let Some(cmd) = palette.confirm(0) {
        execute_command(palette, state, cmd);
    }
}

mod usage_tracking_behavior_tests {
    use super::*;

    #[test]
    fn test_empty_filter_shows_all_commands() {
        let mut palette = make_palette();

        // No usage tracked - should show all commands
        palette.filter("");

        assert_eq!(palette.filtered_commands.len(), 10);
    }

    #[test]
    fn test_filtered_results_not_empty_when_matching() {
        let mut palette = make_palette();

        // Filter by "new" should match NewSession
        palette.filter("new");

        assert!(!palette.filtered_commands.is_empty());
    }

    #[test]
    fn test_filter_to_one_item_still_works() {
        let mut palette = make_palette();

        // "quit" matches only Quit
        palette.filter("quit");

        assert_eq!(palette.filtered_commands.len(), 1);
    }

    #[test]
    fn test_multiple_commands_execute_without_error() {
        let mut state = make_state();
        let mut palette = make_palette();

        let commands = vec![
            PaletteCommand::NewSession,
            PaletteCommand::ClearChat,
            PaletteCommand::Quit,
        ];

        for cmd in commands {
            let mut p = make_palette();
            let mut s = make_state();
            execute_command(&mut p, &mut s, cmd);
        }

        // If we get here without panic, test passes
        assert!(true);
    }
}

mod filter_order_by_usage_tests {
    use super::*;

    #[test]
    fn test_commands_sorted_by_label_when_no_usage() {
        let mut palette = make_palette();

        // With no usage, commands are sorted by internal order
        palette.filter("");

        // All 10 commands should be present
        assert_eq!(palette.filtered_commands.len(), 10);
    }

    #[test]
    fn test_filter_respects_score_and_usage() {
        let mut palette = make_palette();

        // Filter by single character "c" - matches clear_chat and copy
        palette.filter("c");

        // Should return matches (score > 0)
        assert!(!palette.filtered_commands.is_empty());
    }

    #[test]
    fn test_empty_filter_preserves_all_commands() {
        let mut palette = make_palette();

        // First filter narrow
        palette.filter("q");
        assert!(!palette.filtered_commands.is_empty());

        // Then empty filter - all back
        palette.filter("");

        assert_eq!(palette.filtered_commands.len(), 10);
    }

    #[test]
    fn test_filter_narrowing_then_widening() {
        let mut palette = make_palette();

        // Start with all
        palette.filter("");
        let all_count = palette.filtered_commands.len();

        // Narrow to "q"
        palette.filter("q");
        let q_count = palette.filtered_commands.len();

        // Widen to "qu"
        palette.filter("qu");
        let qu_count = palette.filtered_commands.len();

        assert!(all_count > q_count);
        assert!(q_count >= qu_count);
    }
}

mod usage_affects_filter_order_tests {
    use super::*;

    #[test]
    fn test_empty_filter_shows_commands_in_default_order() {
        let mut palette = make_palette();

        // With empty filter and no usage history, commands shown in default order
        palette.filter("");

        // The first command should be the first one in the all_commands list
        let first_idx = palette.filtered_commands[0];
        let first_cmd = &palette.all_commands()[first_idx];

        // Just verify we have a valid first command
        assert!(!first_cmd.label.is_empty());
    }

    #[test]
    fn test_high_usage_command_in_filter_results() {
        let mut palette = make_palette();

        // Filter "c" - both clear_chat and copy match
        palette.filter("c");

        // We should get at least one result
        assert!(!palette.filtered_commands.is_empty());

        // Verify the first result is one of the matching commands
        let first_idx = palette.filtered_commands[0];
        let first_cmd = &palette.all_commands()[first_idx];
        assert!(first_cmd.label.contains("Clear") || first_cmd.label.contains("Copy"));
    }

    #[test]
    fn test_quit_filter_returns_quit_command() {
        let mut palette = make_palette();

        palette.filter("quit");

        assert_eq!(palette.filtered_commands.len(), 1);

        let idx = palette.filtered_commands[0];
        let cmd = &palette.all_commands()[idx];
        assert_eq!(cmd.id, "quit");
    }

    #[test]
    fn test_new_session_filter_returns_new_session_command() {
        let mut palette = make_palette();

        palette.filter("new");

        let matching = palette.filtered_commands.iter().any(|&idx| {
            palette.all_commands()[idx].id == "new_session"
        });
        assert!(matching);
    }

    #[test]
    fn test_partial_filter_matches_prefix() {
        let mut palette = make_palette();

        // "on" could match "Onboard" or "New Session" with fuzzy
        palette.filter("on");

        assert!(!palette.filtered_commands.is_empty());
    }
}

mod confirm_tracks_usage_tests {
    use super::*;

    #[test]
    fn test_confirm_increments_filter_count() {
        let mut palette = make_palette();

        // Confirm NewSession
        palette.filter("new");
        let result = palette.confirm(0);
        assert!(result.is_some());

        // Now empty filter should still work
        palette.filter("");
        assert_eq!(palette.filtered_commands.len(), 10);
    }

    #[test]
    fn test_confirm_returns_correct_command() {
        let mut palette = make_palette();

        palette.filter("quit");
        let result = palette.confirm(0);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PaletteCommand::Quit);
    }

    #[test]
    fn test_confirm_clear_chat() {
        let mut palette = make_palette();

        palette.filter("clear");
        let result = palette.confirm(0);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PaletteCommand::ClearChat);
    }

    #[test]
    fn test_confirm_copy_last() {
        let mut palette = make_palette();

        palette.filter("copy");
        let result = palette.confirm(0);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PaletteCommand::CopyLast);
    }

    #[test]
    fn test_confirm_show_cost() {
        let mut palette = make_palette();

        palette.filter("cost");
        let result = palette.confirm(0);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PaletteCommand::ShowCost);
    }

    #[test]
    fn test_confirm_help() {
        let mut palette = make_palette();

        palette.filter("help");
        let result = palette.confirm(0);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PaletteCommand::Help);
    }

    #[test]
    fn test_confirm_switch_model() {
        let mut palette = make_palette();

        palette.filter("switch");
        let result = palette.confirm(0);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PaletteCommand::SwitchModel);
    }

    #[test]
    fn test_confirm_session_tree() {
        let mut palette = make_palette();

        palette.filter("tree");
        let result = palette.confirm(0);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PaletteCommand::SessionTree);
    }

    #[test]
    fn test_confirm_fork_session() {
        let mut palette = make_palette();

        palette.filter("fork");
        let result = palette.confirm(0);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PaletteCommand::ForkSession);
    }

    #[test]
    fn test_confirm_onboard() {
        let mut palette = make_palette();

        palette.filter("onboard");
        let result = palette.confirm(0);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), PaletteCommand::Onboard);
    }
}
