//! Tests for Tab key opening file picker with smart insertion.
//! 
//! When user presses Enter in file picker:
//! - If cursor at end of input with typed text, REPLACE the typed text
//! - If cursor in middle of input, INSERT at cursor position preserving surrounding text
//! - @ opens file picker but is not part of final text

use crate::{AppState, Event};

/// Typing text then Tab, then Enter replaces the typed text with [filename].
#[test]
fn tab_file_picker_replaces_typed_prefix() {
    let mut state = AppState::default();
    
    // Type some characters
    state.update(Event::Input('T'));
    state.update(Event::Input('e'));
    state.update(Event::Input('s'));
    
    assert_eq!(state.input.input, "Tes");
    
    // Press Tab to open file picker
    state.update(Event::Input('\t'));
    assert!(state.open_dialog.is_some());
    
    // Submit (Enter) to select
    state.update(Event::Submit);
    
    // File picker should close
    assert!(state.open_dialog.is_none());
    
    // The typed prefix should be replaced with [filename]
    assert!(
        state.input.input.starts_with('['),
        "Should start with '[', got: {}",
        state.input.input
    );
    assert!(
        state.input.input.ends_with(']'),
        "Should end with ']', got: {}",
        state.input.input
    );
}

/// Empty input then Tab, then Enter inserts [filename].
#[test]
fn tab_file_picker_inserts_at_empty_position() {
    let mut state = AppState::default();
    
    // Press Tab with empty input
    state.update(Event::Input('\t'));
    assert!(state.open_dialog.is_some());
    
    // Submit
    state.update(Event::Submit);
    
    // File picker should close
    assert!(state.open_dialog.is_none());
    
    // Should have inserted [filename]
    assert!(
        state.input.input.starts_with('['),
        "Should start with '[', got: {}",
        state.input.input
    );
}

/// Cursor in middle of text - inserts [filename] at cursor position.
#[test]
fn tab_file_picker_inserts_at_cursor_middle() {
    let mut state = AppState::default();
    state.input.input = "Hello World".to_string();
    state.input.cursor_pos = 5; // cursor after "Hello"
    
    // Press Tab
    state.update(Event::Input('\t'));
    assert!(state.open_dialog.is_some());
    
    // Submit
    state.update(Event::Submit);
    
    // File picker should close
    assert!(state.open_dialog.is_none());
    
    // Should have inserted [filename] at cursor position
    // "Hello" + [filename] + " World"
    assert!(
        state.input.input.starts_with("Hello["),
        "Should have 'Hello[' prefix, got: {}",
        state.input.input
    );
}

/// Cursor at end of text - replaces trailing prefix with [filename].
#[test]
fn tab_file_picker_replaces_trailing_prefix() {
    let mut state = AppState::default();
    state.input.input = "Hello Tes".to_string();
    state.input.cursor_pos = 9; // cursor at end
    
    // Press Tab
    state.update(Event::Input('\t'));
    assert!(state.open_dialog.is_some());
    
    // Submit
    state.update(Event::Submit);
    
    // File picker should close
    assert!(state.open_dialog.is_none());
    
    // Should have replaced "Tes" with [filename]
    assert!(
        state.input.input.starts_with("Hello["),
        "Should have 'Hello[' prefix, got: {}",
        state.input.input
    );
}

/// @ opens file picker and filter can be typed.
/// Note: @ is consumed when file is selected (not part of final text).
#[test]
fn tab_file_picker_inserts_after_at() {
    let mut state = AppState::default();
    
    // Type @ - opens file picker immediately
    state.update(Event::Input('@'));
    assert!(state.open_dialog.is_some());
    
    // Type more characters to filter
    state.update(Event::Input('C'));
    state.update(Event::Input('a'));
    
    // Submit
    state.update(Event::Submit);
    
    // File picker should close
    assert!(state.open_dialog.is_none());
    
    // Should have inserted [filename] (no @)
    assert!(
        state.input.input.starts_with('['),
        "Should start with '[', got: {}",
        state.input.input
    );
    assert!(
        !state.input.input.contains('@'),
        "Should not contain '@', got: {}",
        state.input.input
    );
}

/// @ without typed text inserts [filename].
#[test]
fn tab_file_picker_only_at_inserts_after_at() {
    let mut state = AppState::default();
    
    // Type just @ - opens file picker
    state.update(Event::Input('@'));
    assert!(state.open_dialog.is_some());
    
    // Submit
    state.update(Event::Submit);
    
    // File picker should close
    assert!(state.open_dialog.is_none());
    
    // Should have inserted [filename]
    assert!(
        state.input.input.starts_with('['),
        "Should start with '[', got: {}",
        state.input.input
    );
}

/// Escape closes file picker and restores original input.
#[test]
fn escape_closes_file_picker_restores_input() {
    let mut state = AppState::default();
    
    // Type some characters
    state.update(Event::Input('T'));
    state.update(Event::Input('e'));
    state.update(Event::Input('s'));
    
    let original_input = state.input.input.clone();
    
    // Press Tab to open file picker
    state.update(Event::Input('\t'));
    assert!(state.open_dialog.is_some());
    
    // Press Escape to close
    state.update(Event::Abort);
    
    // File picker should close
    assert!(state.open_dialog.is_none());
    
    // Input should be restored
    assert_eq!(
        state.input.input, original_input,
        "Input should be restored after Escape, got: {}",
        state.input.input
    );
}

/// Tab cycles through file picker items.
#[test]
fn tab_cycles_file_picker_items() {
    let mut state = AppState::default();
    
    // Press Tab to open file picker
    state.update(Event::Input('\t'));
    assert!(state.open_dialog.is_some());
    
    let initial_selection = get_panel_selection(&state);
    
    // Second Tab cycles to next
    state.update(Event::Input('\t'));
    let next_selection = get_panel_selection(&state);
    
    // Should have moved to next (or wrapped)
    if get_panel_items_count(&state) > 1 {
        assert_ne!(
            initial_selection, next_selection,
            "Tab should change selection"
        );
    }
}

// Helper functions

fn get_panel_selection(state: &AppState) -> usize {
    use crate::commands::DialogState;
    if let Some(DialogState::PanelStack(stack)) = &state.open_dialog {
        if let Some(panel) = stack.current() {
            return panel.selected;
        }
    }
    0
}

fn get_panel_items_count(state: &AppState) -> usize {
    use crate::commands::DialogState;
    if let Some(DialogState::PanelStack(stack)) = &state.open_dialog {
        if let Some(panel) = stack.current() {
            return panel.items.len();
        }
    }
    0
}
