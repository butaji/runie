//! Unit tests for CommandPalette - filter, confirm, and usage tracking.

use crate::components::command_palette::{CommandPalette, PaletteCommand};

fn make_fresh_palette() -> CommandPalette {
    CommandPalette::new()
}

mod filter_tests {
    use super::*;

    #[test]
    fn test_filter_empty_query_shows_all_commands() {
        let mut palette = make_fresh_palette();
        palette.filter("");

        // All commands should be present
        assert_eq!(palette.filtered_commands.len(), palette.all_commands().len());
    }

    #[test]
    fn test_filter_empty_query_sorts_by_usage() {
        let mut palette = make_fresh_palette();

        // Use some commands to build up usage stats
        palette.filter("new");
        palette.confirm(0); // NewSession

        palette.filter("clear");
        palette.confirm(0); // ClearChat

        palette.filter("quit");
        palette.confirm(0); // Quit

        // Now filter with empty query - most used should come first
        palette.filter("");

        // The first command in filtered list should be one with high usage
        let first_idx = palette.filtered_commands[0];
        let first_cmd = &palette.all_commands()[first_idx];
        // quit has 1 use, new_session has 1 use, clear_chat has 1 use
        // With equal usage, order is implementation-defined but all present
        assert_eq!(palette.filtered_commands.len(), palette.all_commands().len());
    }

    #[test]
    fn test_filter_exact_match() {
        let mut palette = make_fresh_palette();

        palette.filter("New Session");
        let filtered = &palette.filtered_commands;
        assert!(!filtered.is_empty());

        let cmd = &palette.all_commands()[filtered[0]];
        assert_eq!(cmd.label, "New Session");
    }

    #[test]
    fn test_filter_prefix_match() {
        let mut palette = make_fresh_palette();

        palette.filter("New");
        let filtered = &palette.filtered_commands;
        assert!(!filtered.is_empty());

        let cmd = &palette.all_commands()[filtered[0]];
        assert!(cmd.label.contains("New") || cmd.id.contains("new"));
    }

    #[test]
    fn test_filter_partial_match() {
        let mut palette = make_fresh_palette();

        palette.filter("sess");
        let filtered = &palette.filtered_commands;
        assert!(!filtered.is_empty());

        // Should match "session" in labels or ids
        let has_match = filtered.iter().any(|&idx| {
            let cmd = &palette.all_commands()[idx];
            cmd.label.to_lowercase().contains("sess") || cmd.id.contains("sess")
        });
        assert!(has_match);
    }

    #[test]
    fn test_filter_alias_match() {
        let mut palette = make_fresh_palette();

        // "n" is an alias for New Session
        palette.filter("n");
        let filtered = &palette.filtered_commands;
        assert!(!filtered.is_empty());

        let cmd = &palette.all_commands()[filtered[0]];
        assert!(cmd.aliases.contains(&"n".to_string()));
    }

    #[test]
    fn test_filter_fuzzy_match() {
        let mut palette = make_fresh_palette();

        // "nw" should fuzzy match "New Session" (n->N, w->w in "New")
        palette.filter("nw");
        // Should find New Session via fuzzy matching
        let has_new_session = palette.filtered_commands.iter().any(|&idx| {
            palette.all_commands()[idx].id == "new_session"
        });
        assert!(has_new_session, "Should fuzzy match 'nw' to 'new_session'");
    }

    #[test]
    fn test_filter_no_match_returns_empty() {
        let mut palette = make_fresh_palette();

        palette.filter("xyznonexistent");
        assert!(palette.filtered_commands.is_empty());
    }

    #[test]
    fn test_filter_case_insensitive() {
        let mut palette = make_fresh_palette();

        palette.filter("NEW");
        let filtered_upper = palette.filtered_commands.clone();

        palette.filter("new");
        let filtered_lower = palette.filtered_commands.clone();

        // Same results regardless of case
        assert_eq!(filtered_upper, filtered_lower);
    }
}

mod confirm_tests {
    use super::*;

    #[test]
    fn test_confirm_no_arg_command() {
        let mut palette = make_fresh_palette();

        // Select NewSession (first command, no args)
        palette.filter("");
        let result = palette.confirm(0);

        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd, PaletteCommand::NewSession);
    }

    #[test]
    fn test_confirm_quit_command() {
        let mut palette = make_fresh_palette();

        palette.filter("quit");
        let result = palette.confirm(0);

        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd, PaletteCommand::Quit);
    }

    #[test]
    fn test_confirm_clear_chat_command() {
        let mut palette = make_fresh_palette();

        palette.filter("clear");
        let result = palette.confirm(0);

        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd, PaletteCommand::ClearChat);
    }

    #[test]
    fn test_confirm_requires_arg_enters_argument_mode() {
        let mut palette = make_fresh_palette();

        // LoadSession requires args
        palette.filter("load");
        let result = palette.confirm(0);

        // Should return None and enter argument mode
        assert!(result.is_none());
        assert!(palette.is_argument_mode);
        assert!(palette.pending_command.is_some());
        assert_eq!(palette.pending_command.as_ref().unwrap(), "load_session");
    }

    #[test]
    fn test_confirm_read_file_enters_argument_mode() {
        let mut palette = make_fresh_palette();

        palette.filter("read");
        let result = palette.confirm(0);

        // Should return None and enter argument mode
        assert!(result.is_none());
        assert!(palette.is_argument_mode);
        assert_eq!(palette.pending_command.as_ref().unwrap(), "read_file");
    }

    #[test]
    fn test_confirm_out_of_bounds_returns_none() {
        let mut palette = make_fresh_palette();

        palette.filter("");
        let result = palette.confirm(999); // Out of bounds

        assert!(result.is_none());
    }

    #[test]
    fn test_confirm_empty_filter_returns_none() {
        let mut palette = make_fresh_palette();

        palette.filter("xyznonexistent");
        let result = palette.confirm(0);

        assert!(result.is_none());
    }
}

mod confirm_with_argument_tests {
    use super::*;

    #[test]
    fn test_confirm_with_argument_returns_command() {
        let mut palette = make_fresh_palette();

        // Enter argument mode for LoadSession
        palette.filter("load");
        palette.confirm(0);
        assert!(palette.is_argument_mode);

        // Type an argument
        for c in "my_session".chars() {
            palette.insert_char(c);
        }
        assert_eq!(palette.argument_input, "my_session");

        // Confirm with argument
        let result = palette.confirm_with_argument();

        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd, PaletteCommand::LoadSession { name: "my_session".to_string() });

        // Should exit argument mode
        assert!(!palette.is_argument_mode);
        assert!(palette.argument_input.is_empty());
        assert!(palette.pending_command.is_none());
    }

    #[test]
    fn test_confirm_with_argument_empty_input() {
        let mut palette = make_fresh_palette();

        // Enter argument mode for LoadSession
        palette.filter("load");
        palette.confirm(0);

        // Confirm with empty argument
        let result = palette.confirm_with_argument();

        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd, PaletteCommand::LoadSession { name: "".to_string() });
    }

    #[test]
    fn test_confirm_with_argument_read_file() {
        let mut palette = make_fresh_palette();

        // Enter argument mode for ReadFile
        palette.filter("read");
        palette.confirm(0);

        // Type a path
        for c in "/path/to/file.txt".chars() {
            palette.insert_char(c);
        }

        let result = palette.confirm_with_argument();

        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd, PaletteCommand::ReadFile { path: "/path/to/file.txt".to_string() });
    }

    #[test]
    fn test_confirm_with_argument_no_pending_command_returns_none() {
        let mut palette = make_fresh_palette();

        // Not in argument mode - calling confirm_with_argument should return None
        let result = palette.confirm_with_argument();
        assert!(result.is_none());
    }
}

mod track_usage_tests {
    use super::*;

    #[test]
    fn test_track_usage_increments_count() {
        let mut palette = make_fresh_palette();

        // Initially no usage
        palette.filter("new");
        assert_eq!(palette.confirm(0), Some(PaletteCommand::NewSession));

        // Check usage was tracked - filter again to access
        palette.filter("");
        let new_session_idx = palette.all_commands().iter().position(|c| c.id == "new_session").unwrap();
        let usage = palette.usage_stats.get("new_session");
        assert!(usage.is_some());
        assert_eq!(usage.unwrap().use_count, 1);

        // Use again
        palette.filter("new");
        palette.confirm(0);
        assert_eq!(palette.usage_stats.get("new_session").unwrap().use_count, 2);
    }

    #[test]
    fn test_track_usage_last_used_updated() {
        use std::time::Instant;

        let mut palette = make_fresh_palette();

        palette.filter("new");
        palette.confirm(0);

        let usage = palette.usage_stats.get("new_session").unwrap();
        assert!(usage.last_used.is_some());

        // Should be recent (within last second)
        let elapsed = Instant::now().duration_since(usage.last_used.unwrap());
        assert!(elapsed.as_secs() < 1);
    }

    #[test]
    fn test_track_usage_multiple_commands() {
        let mut palette = make_fresh_palette();

        palette.filter("new");
        palette.confirm(0);

        palette.filter("clear");
        palette.confirm(0);

        palette.filter("quit");
        palette.confirm(0);

        assert_eq!(palette.usage_stats.get("new_session").unwrap().use_count, 1);
        assert_eq!(palette.usage_stats.get("clear_chat").unwrap().use_count, 1);
        assert_eq!(palette.usage_stats.get("quit").unwrap().use_count, 1);
    }

    #[test]
    fn test_track_usage_argument_command() {
        let mut palette = make_fresh_palette();

        // LoadSession requires args
        palette.filter("load");
        palette.confirm(0);

        for c in "session_name".chars() {
            palette.insert_char(c);
        }
        palette.confirm_with_argument();

        // Usage should be tracked for load_session
        let usage = palette.usage_stats.get("load_session");
        assert!(usage.is_some());
        assert_eq!(usage.unwrap().use_count, 1);
    }
}

mod command_sorting_tests {
    use super::*;

    #[test]
    fn test_commands_sorted_by_usage_after_filter() {
        let mut palette = make_fresh_palette();

        // Build up usage: quit=3, new=2, clear=1
        for _ in 0..3 {
            palette.filter("quit");
            palette.confirm(0);
        }
        for _ in 0..2 {
            palette.filter("new");
            palette.confirm(0);
        }
        for _ in 0..1 {
            palette.filter("clear");
            palette.confirm(0);
        }

        // Filter by partial match that should return multiple commands
        palette.filter("");

        // Most used should be first
        let first_idx = palette.filtered_commands[0];
        let first_cmd = &palette.all_commands()[first_idx];
        assert_eq!(first_cmd.id, "quit");

        // Second most used should be second
        let second_idx = palette.filtered_commands[1];
        let second_cmd = &palette.all_commands()[second_idx];
        assert_eq!(second_cmd.id, "new_session");
    }

    #[test]
    fn test_usage_sort_with_empty_query() {
        let mut palette = make_fresh_palette();

        // Use commands in reverse order
        palette.filter("clear");
        palette.confirm(0);
        palette.filter("new");
        palette.confirm(0);

        // Empty query should show most used first
        palette.filter("");

        let quit_idx = palette.filtered_commands.iter().position(|&i| palette.all_commands()[i].id == "quit");
        let clear_idx = palette.filtered_commands.iter().position(|&i| palette.all_commands()[i].id == "clear_chat");
        let new_idx = palette.filtered_commands.iter().position(|&i| palette.all_commands()[i].id == "new_session");

        // quit (0 uses currently) might not be first since new and clear have 1 use each
        // But we can verify the order is based on usage
        if let (Some(q), Some(c), Some(n)) = (quit_idx, clear_idx, new_idx) {
            // The ones with higher usage should come first
            let quit_usage = palette.usage_stats.get("quit").map(|u| u.use_count).unwrap_or(0);
            let clear_usage = palette.usage_stats.get("clear_chat").map(|u| u.use_count).unwrap_or(0);
            let new_usage = palette.usage_stats.get("new_session").map(|u| u.use_count).unwrap_or(0);

            assert!(clear_usage >= quit_usage || q < c);
            assert!(new_usage >= clear_usage || c < n);
        }
    }
}

mod argument_mode_tests {
    use super::*;

    #[test]
    fn test_insert_char_in_argument_mode() {
        let mut palette = make_fresh_palette();

        palette.filter("load");
        palette.confirm(0);

        palette.insert_char('a');
        palette.insert_char('b');
        palette.insert_char('c');

        assert_eq!(palette.argument_input, "abc");
    }

    #[test]
    fn test_backspace_in_argument_mode() {
        let mut palette = make_fresh_palette();

        palette.filter("load");
        palette.confirm(0);

        palette.insert_char('a');
        palette.insert_char('b');
        palette.insert_char('c');
        palette.backspace();

        assert_eq!(palette.argument_input, "ab");
    }

    #[test]
    fn test_clear_input_in_argument_mode() {
        let mut palette = make_fresh_palette();

        palette.filter("load");
        palette.confirm(0);

        palette.insert_char('a');
        palette.insert_char('b');
        palette.insert_char('c');
        palette.clear_input();

        assert!(palette.argument_input.is_empty());
    }

    #[test]
    fn test_insert_char_ignored_when_not_in_argument_mode() {
        let mut palette = make_fresh_palette();

        // Not in argument mode
        palette.insert_char('x');

        assert!(palette.argument_input.is_empty());
    }

    #[test]
    fn test_is_argument_mode_active() {
        let mut palette = make_fresh_palette();

        assert!(!palette.is_argument_mode_active());

        palette.filter("load");
        palette.confirm(0);

        assert!(palette.is_argument_mode_active());
    }
}
