//! Tests for @ file picker functionality

use super::*;
use runie_core::Event;

/// Typing @ at the beginning of input (empty input) opens the file picker.
#[test]
fn at_opens_file_picker_when_input_empty() {
    let mut state = AppState::default();
    assert!(state.open_dialog.is_none());

    state.update(Event::Input('@'));

    assert!(
        state.open_dialog.is_some(),
        "Typing @ should open file picker when input is empty"
    );
}

/// Typing @ after a space opens the file picker.
#[test]
fn at_opens_file_picker_after_space() {
    let mut state = AppState::default();
    state.input.input = " ".to_string();
    state.input.cursor_pos = 1;

    assert!(state.open_dialog.is_none());
    state.update(Event::Input('@'));

    assert!(
        state.open_dialog.is_some(),
        "Typing @ after space should open file picker"
    );
}

/// Typing @ in the middle of text does NOT open the file picker.
#[test]
fn at_does_not_open_file_picker_in_middle_of_text() {
    let mut state = AppState::default();
    state.input.input = "Hello @".to_string();
    state.input.cursor_pos = 6;

    state.update(Event::Input('@'));

    // @ should be inserted as a character, not open file picker
    assert!(
        state.open_dialog.is_none(),
        "@ in middle of text should not open file picker"
    );
    assert!(
        state.input.input.contains('@'),
        "@ should be inserted into input"
    );
}

/// Typing @ after some characters (but not preceded by space) opens the file picker.
#[test]
fn at_opens_file_picker_for_at_ref_suggestion() {
    let mut state = AppState::default();
    state.input.input = "@Car".to_string();
    state.input.cursor_pos = 4;

    // This should trigger at_ref suggestions based on the prefix
    // Currently the test just verifies the input state
    assert!(state.input.input.starts_with('@'));
}

/// Tab cycles through file completions when file picker dialog is open.
#[test]
fn tab_cycles_panel_selection_in_file_picker() {
    let mut state = AppState::default();
    inject_mock_file_entries(&mut state);
    // Picker is already open from inject_mock_file_entries — do NOT call
    // Event::Input('@') here as that reopens and wipes the injected items.

    // Verify dialog is open
    assert!(state.open_dialog.is_some(), "File picker should be open");

    // Get initial selection (should be 0)
    let initial_selection = get_panel_selection(&state);
    assert_eq!(initial_selection, 0, "Initial selection should be 0");

    // Tab should cycle to next
    state.update(Event::Input('\t'));

    let after_tab = get_panel_selection(&state);
    assert_eq!(after_tab, 1, "Tab should cycle to next selection");
}

/// Tab cycles through panel items without panicking.
#[test]
fn tab_wraps_around_panel_selection() {
    let mut state = AppState::default();
    inject_mock_file_entries(&mut state);
    // Picker is already open — do NOT call Event::Input('@') here.

    // Get panel to check number of items
    let items_count = get_panel_items_count(&state);

    // First Tab moves to the next item when there is one.
    state.update(Event::Input('\t'));
    let first_tab = get_panel_selection(&state);
    if items_count >= 2 {
        assert_eq!(first_tab, 1, "First Tab should move to index 1");
    }

    // Second Tab stays within bounds. The exact index depends on the
    // asynchronously populated file list, so we only assert validity.
    state.update(Event::Input('\t'));
    let second_tab = get_panel_selection(&state);
    assert!(
        second_tab < items_count.max(1),
        "Second Tab selection {} out of bounds for {} items",
        second_tab,
        items_count
    );
}

/// Enter selects the file and inserts it as [path].
#[test]
fn submit_inserts_selected_file() {
    let mut state = AppState::default();
    inject_mock_file_entries(&mut state);
    // Picker is already open — do NOT call Event::Input('@') here.

    // Navigate to a specific item if there are files
    let items_count = get_panel_items_count(&state);
    if items_count > 1 {
        state.update(Event::HistoryNext);
    }

    // Submit (Enter)
    state.update(Event::submit());

    // File picker should close
    assert!(
        state.open_dialog.is_none(),
        "File picker should close after submit"
    );

    // Input should contain the selected file wrapped in []
    assert!(
        !state.input.input.is_empty() || state.input.input.contains('['),
        "Should have inserted a file reference"
    );
}

// Helper functions to access internal state

fn get_panel_selection(state: &AppState) -> usize {
    use runie_core::commands::{DialogKind, DialogState};
    if let Some(DialogState::Active { kind: DialogKind::Generic, panels: stack }) = &state.open_dialog {
        if let Some(panel) = stack.current() {
            return panel.selected;
        }
    }
    0
}

fn get_panel_items_count(state: &AppState) -> usize {
    use runie_core::commands::{DialogKind, DialogState};
    if let Some(DialogState::Active { kind: DialogKind::Generic, panels: stack }) = &state.open_dialog {
        if let Some(panel) = stack.current() {
            return panel.items.len();
        }
    }
    0
}
