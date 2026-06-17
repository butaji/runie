//! Tests for Tab key opening file picker with smart insertion.
//!
//! When user presses Enter in file picker:
//! - Cursor at end of input with typed text -> replaces typed text
//! - Cursor in middle of input -> inserts at cursor position preserving surrounding text
//! - @ opens file picker but inserts path without brackets

use crate::event::{ControlEvent, InputEvent};

use crate::{AppState, Event};

/// "ca ca" -> Tab -> pick -> "ca filename" (space preserved, NO brackets).
#[test]
fn tab_file_picker_preserves_space_in_middle() {
    let mut state = AppState::default();
    state.input.input = "ca ca".to_string();
    state.input.cursor_pos = 5; // cursor after "ca ca"

    // Press Tab
    state.update(InputEvent::Input('\t'));
    assert!(state.open_dialog.is_some());

    // Submit
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Result should be "ca filename" - space between "ca" and filename
    assert!(
        state.input.input.starts_with("ca "),
        "Should have 'ca ' prefix, got: {}",
        state.input.input
    );
    // Should NOT have brackets
    assert!(
        !state.input.input.contains('[') && !state.input.input.contains(']'),
        "Should NOT have brackets, got: {}",
        state.input.input
    );
}

/// "car dev" -> Tab -> pick -> "car filename" (space preserved, NO brackets).
#[test]
fn tab_file_picker_preserves_space_before_prefix() {
    let mut state = AppState::default();
    state.input.input = "car dev".to_string();
    state.input.cursor_pos = 7; // cursor after "car dev"

    // Press Tab
    state.update(InputEvent::Input('\t'));
    assert!(state.open_dialog.is_some());

    // Submit
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Result should be "car filename" - space between "car" and filename
    assert!(
        state.input.input.starts_with("car "),
        "Should start with 'car ', got: {}",
        state.input.input
    );
    // Should NOT be concatenated like "cardev"
    assert!(
        !state.input.input.starts_with("cardev"),
        "Should NOT be 'cardev', got: {}",
        state.input.input
    );
    // Should NOT have brackets
    assert!(
        !state.input.input.contains('[') && !state.input.input.contains(']'),
        "Should NOT have brackets, got: {}",
        state.input.input
    );
}

/// Empty input -> Tab -> pick -> "filename" (NO brackets).
#[test]
fn tab_file_picker_empty_input() {
    let mut state = AppState::default();
    // Empty input
    assert_eq!(state.input.input, "");

    // Press Tab
    state.update(InputEvent::Input('\t'));
    assert!(state.open_dialog.is_some());

    // Submit
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Result should just be the filename, NO brackets
    assert!(
        !state.input.input.is_empty(),
        "Should have inserted filename"
    );
    assert!(
        !state.input.input.starts_with('['),
        "Should NOT start with '[', got: {}",
        state.input.input
    );
}

/// "Tes" -> Tab -> pick -> replaces "Tes" with filename (NO brackets).
#[test]
fn tab_file_picker_replaces_typed_prefix() {
    let mut state = AppState::default();
    state.input.input = "Tes".to_string();
    state.input.cursor_pos = 3;

    // Press Tab
    state.update(InputEvent::Input('\t'));
    assert!(state.open_dialog.is_some());

    // Submit
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Should NOT start with "Tes"
    assert!(
        !state.input.input.starts_with("Tes"),
        "Should NOT start with 'Tes', got: {}",
        state.input.input
    );
    // Should NOT have brackets
    assert!(
        !state.input.input.contains('[') && !state.input.input.contains(']'),
        "Should NOT have brackets, got: {}",
        state.input.input
    );
}

/// "Hello World" cursor at end -> Tab -> pick -> "Hello filename" (replaces last word).
#[test]
fn tab_file_picker_replaces_last_word_at_end() {
    let mut state = AppState::default();
    state.input.input = "Hello World".to_string();
    state.input.cursor_pos = 11; // cursor after "Hello World"

    // Press Tab
    state.update(InputEvent::Input('\t'));
    assert!(state.open_dialog.is_some());

    // Submit
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Should start with "Hello " and NOT "HelloWorld"
    assert!(
        state.input.input.starts_with("Hello "),
        "Should start with 'Hello ', got: {}",
        state.input.input
    );
    // Should NOT be concatenated
    assert!(
        !state.input.input.starts_with("HelloWorld"),
        "Should NOT be 'HelloWorld', got: {}",
        state.input.input
    );
}

/// "Hello World" cursor in middle -> Tab -> pick -> inserts at cursor.
#[test]
fn tab_file_picker_inserts_at_cursor_middle() {
    let mut state = AppState::default();
    state.input.input = "Hello World".to_string();
    state.input.cursor_pos = 5; // cursor after "Hello"

    // Press Tab
    state.update(InputEvent::Input('\t'));
    assert!(state.open_dialog.is_some());

    // Submit
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Should have filename between "Hello" and " World"
    assert!(
        state.input.input.starts_with("Hello"),
        "Should start with 'Hello', got: {}",
        state.input.input
    );
    assert!(
        state.input.input.ends_with("World"),
        "Should end with 'World', got: {}",
        state.input.input
    );
}

/// "@" alone -> Tab -> pick -> "filename" (NO brackets).
#[test]
fn tab_file_picker_at_alone_no_brackets() {
    let mut state = AppState::default();

    // Type just @
    state.update(InputEvent::Input('@'));
    assert!(state.open_dialog.is_some());

    // Submit
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Should have inserted filename WITHOUT brackets
    assert!(
        !state.input.input.starts_with('['),
        "Should NOT start with '[', got: {}",
        state.input.input
    );
}

/// "car @" -> Tab -> pick -> "car filename" (NO brackets).
#[test]
fn tab_file_picker_at_with_prefix_no_brackets() {
    let mut state = AppState::default();

    // Type "car @"
    state.update(InputEvent::Input('c'));
    state.update(InputEvent::Input('a'));
    state.update(InputEvent::Input('r'));
    state.update(InputEvent::Input(' '));
    state.update(InputEvent::Input('@'));

    // @ opens file picker
    assert!(state.open_dialog.is_some());

    // Submit
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Should have inserted path WITHOUT brackets
    assert!(
        state.input.input.starts_with("car "),
        "Should start with 'car ', got: {}",
        state.input.input
    );
    assert!(
        !state.input.input.contains('[') && !state.input.input.contains(']'),
        "Should NOT contain brackets, got: {}",
        state.input.input
    );
}

/// Escape closes file picker and restores original input.
#[test]
fn escape_closes_file_picker_restores_input() {
    let mut state = AppState::default();

    // Type some characters
    state.update(InputEvent::Input('T'));
    state.update(InputEvent::Input('e'));
    state.update(InputEvent::Input('s'));

    let original_input = state.input.input.clone();

    // Press Tab to open file picker
    state.update(InputEvent::Input('\t'));
    assert!(state.open_dialog.is_some());

    // Press Escape to close
    state.update(ControlEvent::Abort);

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Input should be restored
    assert_eq!(
        state.input.input, original_input,
        "Input should be restored after Escape, got: {}",
        state.input.input
    );
}

/// "test  " (trailing spaces) -> Tab -> pick -> preserves trailing space.
#[test]
fn tab_file_picker_preserves_trailing_space() {
    let mut state = AppState::default();
    state.input.input = "test  ".to_string(); // with trailing space
    state.input.cursor_pos = 5;

    // Press Tab
    state.update(InputEvent::Input('\t'));
    assert!(state.open_dialog.is_some());

    // Submit
    state.update(Event::submit());

    // File picker should close
    assert!(state.open_dialog.is_none());

    // Should have space after "test" and NO brackets
    assert!(
        state.input.input.starts_with("test "),
        "Should start with 'test ', got: {}",
        state.input.input
    );
    assert!(
        !state.input.input.contains('['),
        "Should NOT have brackets, got: {}",
        state.input.input
    );
}
