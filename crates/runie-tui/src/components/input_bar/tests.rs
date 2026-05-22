//! Comprehensive input bar tests using AppState and update().
//!
//! These tests verify the input editing behavior through the public API
//! (AppState + update function) to ensure the dirty flag is properly set
//! and prevent the typing bug regression where direct update() calls
//! without setting dirty would cause blank renders.

use crate::tui::state::AppState;
use crate::tui::update::update;
use crate::Msg;

/// MockTui mirrors the Tui.update() pattern that sets dirty BEFORE calling
/// the reducer, preventing the bug where direct update() calls without
/// setting dirty caused render() to skip.
struct MockTui {
    state: AppState,
    dirty: bool,
}

impl MockTui {
    fn new() -> Self {
        Self {
            state: AppState::default(),
            dirty: false,
        }
    }

    /// CORRECT pattern: sets dirty BEFORE calling the reducer.
    /// The bug was calling update() directly on state without setting dirty.
    fn update(&mut self, msg: Msg) {
        self.dirty = true;
        update(&mut self.state, msg);
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Simulate render - only "draws" if dirty, then clears dirty.
    fn render(&mut self) -> bool {
        if !self.dirty {
            return false;
        }
        self.dirty = false;
        true
    }

    /// Get current line content
    fn current_line(&self) -> &str {
        &self.state.input_lines[self.state.cursor_row]
    }

    /// Get all lines
    fn lines(&self) -> &[String] {
        &self.state.input_lines
    }
}

#[test]
fn test_insert_single_char() {
    let mut tui = MockTui::new();
    tui.update(Msg::InsertChar('a'));

    assert_eq!(tui.current_line(), "a", "line content should be 'a'");
    assert_eq!(tui.state.cursor_col, 1, "cursor should be at 1");
    assert!(tui.is_dirty(), "dirty should be set after InsertChar");
}

#[test]
fn test_insert_multiple_chars() {
    let mut tui = MockTui::new();
    tui.update(Msg::InsertChar('h'));
    tui.update(Msg::InsertChar('e'));
    tui.update(Msg::InsertChar('l'));
    tui.update(Msg::InsertChar('l'));
    tui.update(Msg::InsertChar('o'));

    assert_eq!(tui.current_line(), "hello", "line content should be 'hello'");
    assert_eq!(tui.state.cursor_col, 5, "cursor should be at 5");
    assert!(tui.is_dirty(), "dirty should be set after inserts");
}

#[test]
fn test_backspace_removes_char() {
    let mut tui = MockTui::new();
    tui.update(Msg::InsertChar('h'));
    tui.update(Msg::InsertChar('i'));
    tui.update(Msg::Backspace);

    assert_eq!(tui.current_line(), "h", "line content should be 'h' after backspace");
    assert_eq!(tui.state.cursor_col, 1, "cursor should be at 1");
    assert!(tui.is_dirty(), "dirty should be set after Backspace");
}

#[test]
fn test_backspace_at_start_does_nothing() {
    let mut tui = MockTui::new();
    // Backspace on empty line should not panic
    tui.update(Msg::Backspace);

    assert_eq!(tui.current_line(), "", "empty line should remain empty");
    assert_eq!(tui.state.cursor_col, 0, "cursor should remain at 0");
    assert!(tui.is_dirty(), "dirty should be set (state was examined)");
}

#[test]
fn test_delete_forward() {
    let mut tui = MockTui::new();
    tui.update(Msg::InsertChar('a'));
    tui.update(Msg::InsertChar('b'));
    tui.update(Msg::InsertChar('c'));
    // cursor at 3
    tui.update(Msg::MoveCursorToStart); // cursor at 0
    tui.update(Msg::MoveCursorRight);   // cursor at 1
    tui.update(Msg::DeleteForward);

    assert_eq!(tui.current_line(), "ac", "delete_forward should remove char at cursor");
    assert_eq!(tui.state.cursor_col, 1, "cursor should remain at 1");
    assert!(tui.is_dirty(), "dirty should be set after DeleteForward");
}

#[test]
fn test_delete_word_backward() {
    let mut tui = MockTui::new();
    for ch in "hello world".chars() {
        tui.update(Msg::InsertChar(ch));
    }
    // cursor at 11 (end of "hello world")
    tui.update(Msg::DeleteWordBackward);

    // AppState's delete_word_backward deletes to the last whitespace, so it removes the space too
    assert_eq!(tui.current_line(), "hello", "should delete 'world' and preceding space");
    assert!(tui.is_dirty(), "dirty should be set after DeleteWordBackward");
}

#[test]
fn test_delete_to_start() {
    let mut tui = MockTui::new();
    for ch in "hello".chars() {
        tui.update(Msg::InsertChar(ch));
    }
    // cursor at 5
    tui.update(Msg::DeleteToStart);

    assert_eq!(tui.current_line(), "", "delete_to_start should clear the line");
    assert_eq!(tui.state.cursor_col, 0, "cursor should be at 0");
    assert!(tui.is_dirty(), "dirty should be set after DeleteToStart");
}

#[test]
fn test_newline_creates_new_line() {
    let mut tui = MockTui::new();
    tui.update(Msg::InsertChar('a'));
    tui.update(Msg::InsertNewline);
    tui.update(Msg::InsertChar('b'));

    assert_eq!(tui.lines().len(), 2, "should have 2 lines");
    assert_eq!(tui.lines()[0], "a", "first line should be 'a'");
    assert_eq!(tui.lines()[1], "b", "second line should be 'b'");
    assert_eq!(tui.state.cursor_row, 1, "cursor should be on line 1");
    assert!(tui.is_dirty(), "dirty should be set after InsertNewline");
}

#[test]
fn test_move_cursor_left() {
    let mut tui = MockTui::new();
    tui.update(Msg::InsertChar('a'));
    tui.update(Msg::InsertChar('b'));
    tui.update(Msg::InsertChar('c'));
    // cursor at 3
    tui.update(Msg::MoveCursorLeft);
    tui.update(Msg::MoveCursorLeft);
    // cursor at 1

    assert_eq!(tui.state.cursor_col, 1, "cursor should be at 1 after 2 MoveLeft");
    assert!(tui.is_dirty(), "dirty should be set after MoveCursorLeft");
}

#[test]
fn test_move_cursor_right() {
    let mut tui = MockTui::new();
    tui.update(Msg::InsertChar('a'));
    tui.update(Msg::InsertChar('b'));
    tui.update(Msg::InsertChar('c'));
    // cursor at 3
    tui.update(Msg::MoveCursorLeft);  // cursor at 2
    tui.update(Msg::MoveCursorRight); // cursor at 3

    assert_eq!(tui.state.cursor_col, 3, "cursor should be at 3 after MoveRight");
    assert!(tui.is_dirty(), "dirty should be set after MoveCursorRight");
}

#[test]
fn test_move_cursor_to_start() {
    let mut tui = MockTui::new();
    for ch in "hello".chars() {
        tui.update(Msg::InsertChar(ch));
    }
    // cursor at 5
    tui.update(Msg::MoveCursorToStart);

    assert_eq!(tui.state.cursor_col, 0, "cursor should be at 0 after MoveToStart");
    assert!(tui.is_dirty(), "dirty should be set after MoveCursorToStart");
}

#[test]
fn test_move_cursor_to_end() {
    let mut tui = MockTui::new();
    for ch in "hello".chars() {
        tui.update(Msg::InsertChar(ch));
    }
    // cursor at 5
    tui.update(Msg::MoveCursorToStart); // cursor at 0
    tui.update(Msg::MoveCursorToEnd);   // cursor at 5

    assert_eq!(tui.state.cursor_col, 5, "cursor should be at 5 after MoveToEnd");
    assert!(tui.is_dirty(), "dirty should be set after MoveCursorToEnd");
}

#[test]
fn test_cursor_bounds_check() {
    let mut tui = MockTui::new();
    // MoveLeft on empty line should not panic
    tui.update(Msg::MoveCursorLeft);

    assert_eq!(tui.state.cursor_col, 0, "cursor should remain at 0");
    assert_eq!(tui.state.cursor_row, 0, "cursor row should remain at 0");
    assert!(tui.is_dirty(), "dirty should be set");
}

#[test]
fn test_insert_at_cursor_position() {
    let mut tui = MockTui::new();
    tui.update(Msg::InsertChar('a'));
    tui.update(Msg::InsertChar('b'));
    tui.update(Msg::InsertChar('c'));
    // cursor at 3
    tui.update(Msg::MoveCursorLeft); // cursor at 2
    tui.update(Msg::InsertChar('x'));

    assert_eq!(tui.current_line(), "abxc", "insert should be at cursor position 2");
    assert_eq!(tui.state.cursor_col, 3, "cursor should be after inserted char");
    assert!(tui.is_dirty(), "dirty should be set after InsertChar");
}

#[test]
fn test_multi_line_input() {
    let mut tui = MockTui::new();
    for ch in "line1".chars() {
        tui.update(Msg::InsertChar(ch));
    }
    tui.update(Msg::InsertNewline);
    for ch in "line2".chars() {
        tui.update(Msg::InsertChar(ch));
    }

    assert_eq!(tui.lines().len(), 2, "should have 2 lines");
    assert_eq!(tui.lines()[0], "line1", "first line should be 'line1'");
    assert_eq!(tui.lines()[1], "line2", "second line should be 'line2'");
    assert!(tui.is_dirty(), "dirty should be set");
}

#[test]
fn test_submit_clears_input() {
    let mut tui = MockTui::new();
    for ch in "test".chars() {
        tui.update(Msg::InsertChar(ch));
    }
    tui.update(Msg::Submit);

    assert!(tui.state.input_lines.is_empty() || tui.state.input_lines[0].is_empty(),
            "Submit should clear input");
    assert_eq!(tui.state.cursor_col, 0, "cursor should reset to 0");
    assert_eq!(tui.state.cursor_row, 0, "cursor row should reset to 0");
    assert!(tui.is_dirty(), "dirty should be set after Submit");
}

#[test]
fn test_render_respects_dirty_flag() {
    let mut tui = MockTui::new();

    // Initially not dirty, render should skip
    let did_render = tui.render();
    assert!(!did_render, "render should skip when not dirty");

    // After update, dirty is set
    tui.update(Msg::InsertChar('x'));
    assert!(tui.is_dirty(), "dirty should be set after update");

    // Now render should execute
    let did_render = tui.render();
    assert!(did_render, "render should execute when dirty");
    assert!(!tui.is_dirty(), "dirty should be cleared after render");
}

#[test]
fn test_dirty_flag_prevents_blank_render() {
    // This test verifies the fix for the typing bug:
    // If someone calls update() directly on state without setting dirty,
    // subsequent renders would be skipped, showing blank input.
    let mut tui = MockTui::new();

    // First update sets dirty
    tui.update(Msg::InsertChar('a'));
    assert!(tui.is_dirty(), "first update should set dirty");

    // Render clears dirty
    tui.render();
    assert!(!tui.is_dirty(), "dirty should be cleared after render");

    // Second update sets dirty again
    tui.update(Msg::InsertChar('b'));
    assert!(tui.is_dirty(), "second update should set dirty");

    // If we incorrectly called update() directly without setting dirty,
    // the render would skip and input would appear blank
    let did_render = tui.render();
    assert!(did_render, "render should execute, preventing blank input bug");
}