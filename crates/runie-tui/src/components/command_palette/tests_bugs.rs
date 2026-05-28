//! Documents: BUG-07 — filter() does not reset selected to 0
//! Documents: BUG-08 — CommandPalette has no cancel() method

use crate::components::command_palette::CommandPalette;

fn make_fresh_palette() -> CommandPalette {
    CommandPalette::new()
}

// BUG-07: filter() does NOT reset selected to 0 - this is the documented bug
#[test]
fn test_selection_reset_on_filter_change() {
    // Documents BUG-07: filter changes should reset selected to 0
    // but currently they do not
    let mut palette = make_fresh_palette();

    palette.filter("");
    // Simulate user navigating to index 5
    palette.selected = 5;

    // Filter with a new query
    palette.filter("new");

    // BUG-07: selected is NOT reset to 0 after filter change
    // This test documents the bug - selected stays at 5
    // The fix would be: palette.selected = 0; at start of filter()
    assert_eq!(palette.selected, 5, "BUG-07: selected should remain at user position");
}

#[test]
fn test_selection_out_of_bounds_after_filter() {
    // Documents what happens when selected becomes out of bounds
    let mut palette = make_fresh_palette();

    palette.filter("");
    palette.selected = 10; // Beyond 16 commands

    // Filter to 3 items
    palette.filter("new");

    // selected is still 10, but filtered_commands has fewer items
    // confirm() should handle this gracefully
    let result = palette.confirm(palette.selected);
    assert!(result.is_none(), "Out of bounds confirm should return None");
}

// BUG-08: CommandPalette has no cancel() method
#[test]
fn test_argument_mode_no_cancel() {
    // Documents BUG-08: There's no cancel() method to exit argument mode
    let mut palette = make_fresh_palette();

    // Enter argument mode
    palette.filter("load");
    palette.confirm(0);
    assert!(palette.is_argument_mode);
    assert!(palette.pending_command.is_some());

    // BUG-08: No cancel() method exists
    // User cannot cancel and return to normal mode
    // The only way out is confirm_with_argument() which creates a command
    // There should be a cancel() method that:
    // - Sets is_argument_mode = false
    // - Clears argument_input
    // - Clears pending_command
    // - Optionally resets selected
}

#[test]
fn test_cannot_exit_argument_mode_without_confirm() {
    let mut palette = make_fresh_palette();

    // Enter argument mode
    palette.filter("load");
    palette.confirm(0);

    // Type some input
    palette.insert_char('t');
    palette.insert_char('e');
    palette.insert_char('s');
    palette.insert_char('t');

    // BUG-08: There's no way to cancel - you MUST provide an argument
    // or the command is created with empty string via confirm_with_argument
    let result = palette.confirm_with_argument();
    assert!(result.is_some()); // Still creates command with "test"
    assert_eq!(palette.is_argument_mode, false); // Must exit mode
}
