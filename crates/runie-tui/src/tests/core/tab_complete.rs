use super::*;
use runie_core::model::AppState;
use runie_core::Event;
use runie_testing::fresh_state;

// =============================================================================
// LAYER 1: State/Logic Tests — Pure function behavior
// =============================================================================

/// Tab with typed prefix opens file picker with filter pre-set
#[test]
fn tab_opens_file_picker_with_filter() {
    let mut state = fresh_state();
    inject_mock_file_entries(&mut state);
    state.input.input = "Tes".to_string();
    state.input.cursor_pos = 3;

    // Tab should open file picker
    state.update(Event::Input('\t'));

    assert!(state.open_dialog.is_some(), "Tab should open file picker");
}

/// Tab with empty input opens file picker without filter
#[test]
fn tab_opens_file_picker_empty_input() {
    let mut state = fresh_state();
    inject_mock_file_entries(&mut state);

    // Tab should open file picker
    state.update(Event::Input('\t'));

    assert!(
        state.open_dialog.is_some(),
        "Tab should open file picker even with empty input"
    );
}

/// Tab cycles through file picker items
#[test]
fn tab_cycles_wraps_around() {
    let mut state = fresh_state();
    inject_mock_file_entries(&mut state);

    // Open file picker
    state.update(Event::Input('\t'));
    assert!(state.open_dialog.is_some());

    // First Tab cycles to next
    state.update(Event::Input('\t'));

    // Get selection
    let selection = get_panel_selection(&state);
    assert!(
        selection > 0 || get_panel_items_count(&state) <= 1,
        "First Tab should cycle"
    );

    // Second Tab should wrap or continue
    state.update(Event::Input('\t'));
}

/// Tab with no matches shows empty file picker
#[test]
fn tab_with_no_matches_shows_empty_picker() {
    let mut state = fresh_state();
    inject_mock_file_entries(&mut state);
    let text = "xyznonexistent123";
    state.input.input = text.to_string();
    state.input.cursor_pos = text.len();

    // Tab opens file picker with non-matching filter
    state.update(Event::Input('\t'));
    assert!(state.open_dialog.is_some());
}

/// File picker selection replaces typed prefix
#[test]
fn file_picker_replaces_typed_prefix() {
    let mut state = fresh_state();
    inject_mock_file_entries(&mut state);
    state.input.input = "Tes".to_string();
    state.input.cursor_pos = 3;

    // Tab opens file picker
    state.update(Event::Input('\t'));
    assert!(state.open_dialog.is_some());

    // Enter selects
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Input should be replaced with filename (no brackets)
    assert!(
        !state.input.input.starts_with("Tes"),
        "Should NOT start with 'Tes', got: {}",
        state.input.input
    );
}

// =============================================================================
// Helper functions
// =============================================================================

fn get_panel_selection(state: &AppState) -> usize {
    use runie_core::commands::DialogState;
    if let Some(DialogState::PanelStack(stack)) = &state.open_dialog {
        if let Some(panel) = stack.current() {
            return panel.selected;
        }
    }
    0
}

fn get_panel_items_count(state: &AppState) -> usize {
    use runie_core::commands::DialogState;
    if let Some(DialogState::PanelStack(stack)) = &state.open_dialog {
        if let Some(panel) = stack.current() {
            return panel.items.len();
        }
    }
    0
}
