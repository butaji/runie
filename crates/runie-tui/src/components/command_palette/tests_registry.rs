//! Tests for command registry - count and requires_args

use crate::components::command_palette::CommandPalette;

fn make_fresh_palette() -> CommandPalette {
    CommandPalette::new()
}

#[test]
fn test_command_count() {
    // Verify all 17 commands are registered
    let palette = make_fresh_palette();
    assert_eq!(palette.all_commands().len(), 17);
}

#[test]
fn test_requires_args_commands() {
    // Verify these commands require args
    let palette = make_fresh_palette();

    let requires_args_ids = [
        "load_session",
        "save_session",
        "read_file",
        "edit_file",
        "write_file",
        "delete_file",
    ];

    for id in requires_args_ids {
        let cmd = palette.all_commands().iter().find(|c| c.id == id).unwrap();
        assert!(cmd.requires_args, "{} should require args", id);
    }
}

#[test]
fn test_no_args_commands() {
    // Verify these commands do NOT require args
    let palette = make_fresh_palette();

    let no_args_ids = [
        "new_session",
        "clear_chat",
        "switch_model",
        "compact_context",
        "quit",
        "manage_providers",
        "add_provider",
        "remove_provider",
        "edit_api_key",
        "set_provider_priority",
        "browse_models",
    ];

    for id in no_args_ids {
        let cmd = palette.all_commands().iter().find(|c| c.id == id).unwrap();
        assert!(!cmd.requires_args, "{} should NOT require args", id);
    }
}

#[test]
fn test_filter_no_matches() {
    // Filter with no matches returns empty list
    let mut palette = make_fresh_palette();
    palette.filter("xyznonexistent");
    assert!(palette.filtered_commands.is_empty());
}

#[test]
fn test_empty_query_sorts_by_usage() {
    // Empty filter shows all, sorted by use_count DESC
    let mut palette = make_fresh_palette();

    // Build up usage: quit=3, new_session=2, clear_chat=1
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

    // Empty query should sort by usage
    palette.filter("");

    let first_idx = palette.filtered_commands[0];
    let first_cmd = &palette.all_commands()[first_idx];
    assert_eq!(first_cmd.id, "quit", "Most used (quit) should be first");

    let second_idx = palette.filtered_commands[1];
    let second_cmd = &palette.all_commands()[second_idx];
    assert_eq!(second_cmd.id, "new_session", "Second most used should be second");
}
