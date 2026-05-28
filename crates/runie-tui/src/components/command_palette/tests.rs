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
        let has_match = filtered.iter().any(|&idx| {
            let cmd = &palette.all_commands()[idx];
            cmd.label.to_lowercase().contains("sess") || cmd.id.contains("sess")
        });
        assert!(has_match);
    }

    #[test]
    fn test_filter_alias_match() {
        let mut palette = make_fresh_palette();
        palette.filter("n");
        let filtered = &palette.filtered_commands;
        assert!(!filtered.is_empty());
        let cmd = &palette.all_commands()[filtered[0]];
        assert!(cmd.aliases.contains(&"n".to_string()));
    }

    #[test]
    fn test_filter_fuzzy_match() {
        let mut palette = make_fresh_palette();
        palette.filter("nw");
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
        assert_eq!(filtered_upper, filtered_lower);
    }
}

mod confirm_tests {
    use super::*;

    #[test]
    fn test_confirm_no_arg_command() {
        let mut palette = make_fresh_palette();
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
    fn test_confirm_requires_arg_enters_argument_mode() {
        let mut palette = make_fresh_palette();
        palette.filter("load");
        let result = palette.confirm(0);
        assert!(result.is_none());
        assert!(palette.is_argument_mode);
        assert!(palette.pending_command.is_some());
        assert_eq!(palette.pending_command.as_ref().unwrap(), "load_session");
    }

    #[test]
    fn test_confirm_out_of_bounds_returns_none() {
        let mut palette = make_fresh_palette();
        palette.filter("");
        let result = palette.confirm(999);
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
        palette.filter("load");
        palette.confirm(0);
        assert!(palette.is_argument_mode);
        for c in "my_session".chars() {
            palette.insert_char(c);
        }
        assert_eq!(palette.argument_input, "my_session");
        let result = palette.confirm_with_argument();
        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd, PaletteCommand::LoadSession { name: "my_session".to_string() });
        assert!(!palette.is_argument_mode);
        assert!(palette.argument_input.is_empty());
        assert!(palette.pending_command.is_none());
    }

    #[test]
    fn test_confirm_with_argument_empty_input() {
        let mut palette = make_fresh_palette();
        palette.filter("load");
        palette.confirm(0);
        let result = palette.confirm_with_argument();
        assert!(result.is_some());
        let cmd = result.unwrap();
        assert_eq!(cmd, PaletteCommand::LoadSession { name: "".to_string() });
    }

    #[test]
    fn test_confirm_with_argument_no_pending_command_returns_none() {
        let mut palette = make_fresh_palette();
        let result = palette.confirm_with_argument();
        assert!(result.is_none());
    }
}

mod track_usage_tests {
    use super::*;

    #[test]
    fn test_track_usage_increments_count() {
        let mut palette = make_fresh_palette();
        palette.filter("new");
        assert_eq!(palette.confirm(0), Some(PaletteCommand::NewSession));
        palette.filter("");
        let usage = palette.usage_stats.get("new_session");
        assert!(usage.is_some());
        assert_eq!(usage.unwrap().use_count, 1);
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
        let elapsed = Instant::now().duration_since(usage.last_used.unwrap());
        assert!(elapsed.as_secs() < 1);
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
