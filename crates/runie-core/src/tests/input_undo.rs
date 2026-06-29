//! Tests for input undo/redo

#[cfg(test)]
mod tests {
    use crate::model::AppState;

    #[test]
    fn undo_reverts_last_insert() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Input('b'));
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "a");
        assert_eq!(state.input.cursor_pos, 1);
    }

    #[test]
    fn undo_reverts_delete_word() {
        let mut state = AppState::default();
        for c in "hello world".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(crate::Event::DeleteWord);
        assert_eq!(state.input.input, "hello ");
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "hello world");
        assert_eq!(state.input.cursor_pos, 11);
    }

    #[test]
    fn redo_restores_undone_action() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Input('b'));
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "a");
        state.update(crate::Event::Redo);
        assert_eq!(state.input.input, "ab");
        assert_eq!(state.input.cursor_pos, 2);
    }

    #[test]
    fn undo_at_empty_does_nothing() {
        let mut state = AppState::default();
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "");
    }

    #[test]
    fn redo_without_undo_does_nothing() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Redo);
        assert_eq!(state.input.input, "a");
    }

    #[test]
    fn typing_clears_redo_stack() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Input('b'));
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "a");
        // Typing clears redo stack
        state.update(crate::Event::Input('c'));
        state.update(crate::Event::Redo);
        assert_eq!(state.input.input, "ac"); // redo had no effect
    }

    #[test]
    fn multiple_undo_steps() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Input('b'));
        state.update(crate::Event::Input('c'));
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "ab");
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "a");
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "");
    }

    #[test]
    fn undo_after_cursor_movement_coalesces() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('a'));
        state.update(crate::Event::Input('b'));
        state.update(crate::Event::CursorLeft);
        state.update(crate::Event::Input('c'));
        // Should be able to undo to before the 'c' insert
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "ab");
        assert_eq!(state.input.cursor_pos, 1);
    }

    #[test]
    fn undo_restores_cursor_position() {
        let mut state = AppState::default();
        for c in "hello".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(crate::Event::CursorStart);
        state.update(crate::Event::DeleteToEnd);
        assert_eq!(state.input.input, "");
        state.update(crate::Event::Undo);
        assert_eq!(state.input.input, "hello");
        assert_eq!(state.input.cursor_pos, 0); // cursor restored to start
    }
}
