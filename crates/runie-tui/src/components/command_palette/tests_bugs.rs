//! Documents: BUG-07 — filter() does not reset selected to 0

use crate::components::command_palette::{CommandPalette, PaletteCommand};

fn make_fresh_palette() -> CommandPalette {
    CommandPalette::new()
}

// BUG-07 FIX TEST: filter() now resets selected to 0
#[test]
fn test_selection_reset_on_filter_change() {
    // BUG-07 was fixed: filter() now resets selected to 0
    let mut palette = make_fresh_palette();

    palette.filter("");
    // Simulate user navigating to index 5
    palette.selected = 5;

    // Filter with a new query
    palette.filter("new");

    // BUG-07 FIX: selected IS reset to 0 after filter change
    assert_eq!(palette.selected, 0, "BUG-07 FIX: selected should reset to 0 on filter change");
}

#[test]
fn test_selection_out_of_bounds_after_filter() {
    // With BUG-07 fix: selected is reset to 0 on filter change
    // So out of bounds is no longer possible via filter change
    let mut palette = make_fresh_palette();

    palette.filter("");
    palette.selected = 10; // Beyond 4 commands

    // Filter with "new" query
    palette.filter("new");

    // BUG-07 FIX: selected resets to 0, so confirm at 0
    assert_eq!(palette.selected, 0);
    // "New Session" (new_session) is first in sorted-by-score results
    // It has requires_args: false, so confirm succeeds
    let result = palette.confirm(palette.selected);
    assert!(matches!(result, Some(PaletteCommand::NewSession)));
}
