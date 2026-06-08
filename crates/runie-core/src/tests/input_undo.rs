//! Tests for input undo/redo

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::event::Event;

    #[test]
    fn undo_reverts_last_insert() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::Undo);
        assert_eq!(state.input, "a");
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn undo_reverts_delete_word() {
        let mut state = AppState::default();
        for c in "hello world".chars() {
            state.update(Event::Input(c));
        }
        state.update(Event::DeleteWord);
        assert_eq!(state.input, "hello ");
        state.update(Event::Undo);
        assert_eq!(state.input, "hello world");
        assert_eq!(state.cursor_pos, 11);
    }

    #[test]
    fn redo_restores_undone_action() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::Undo);
        assert_eq!(state.input, "a");
        state.update(Event::Redo);
        assert_eq!(state.input, "ab");
        assert_eq!(state.cursor_pos, 2);
    }

    #[test]
    fn undo_at_empty_does_nothing() {
        let mut state = AppState::default();
        state.update(Event::Undo);
        assert_eq!(state.input, "");
    }

    #[test]
    fn redo_without_undo_does_nothing() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Redo);
        assert_eq!(state.input, "a");
    }

    #[test]
    fn typing_clears_redo_stack() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::Undo);
        assert_eq!(state.input, "a");
        // Typing clears redo stack
        state.update(Event::Input('c'));
        state.update(Event::Redo);
        assert_eq!(state.input, "ac"); // redo had no effect
    }

    #[test]
    fn multiple_undo_steps() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::Input('c'));
        state.update(Event::Undo);
        assert_eq!(state.input, "ab");
        state.update(Event::Undo);
        assert_eq!(state.input, "a");
        state.update(Event::Undo);
        assert_eq!(state.input, "");
    }

    #[test]
    fn undo_after_cursor_movement_coalesces() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Input('b'));
        state.update(Event::CursorLeft);
        state.update(Event::Input('c'));
        // Should be able to undo to before the 'c' insert
        state.update(Event::Undo);
        assert_eq!(state.input, "ab");
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn undo_restores_cursor_position() {
        let mut state = AppState::default();
        for c in "hello".chars() {
            state.update(Event::Input(c));
        }
        state.update(Event::CursorStart);
        state.update(Event::DeleteToEnd);
        assert_eq!(state.input, "");
        state.update(Event::Undo);
        assert_eq!(state.input, "hello");
        assert_eq!(state.cursor_pos, 0); // cursor restored to start
    }
}
