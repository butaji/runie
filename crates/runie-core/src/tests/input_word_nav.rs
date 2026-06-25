//! Tests for word-level cursor navigation

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::Event;

    #[test]
    fn word_left_to_start_of_previous_word() {
        let mut state = AppState::default();
        for c in "hello world".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(crate::Event::CursorEnd);
        state.update(crate::Event::CursorWordLeft);
        // Should land at start of "world" (position 6)
        assert_eq!(state.input.cursor_pos, 6);
    }

    #[test]
    fn word_left_skips_spaces() {
        let mut state = AppState::default();
        for c in "hello  world".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(crate::Event::CursorEnd);
        state.update(crate::Event::CursorWordLeft);
        assert_eq!(state.input.cursor_pos, 7); // start of "world"
    }

    #[test]
    fn word_left_at_start_does_nothing() {
        let mut state = AppState::default();
        for c in "hi".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(crate::Event::CursorStart);
        state.update(crate::Event::CursorWordLeft);
        assert_eq!(state.input.cursor_pos, 0);
    }

    #[test]
    fn word_right_to_start_of_next_word() {
        let mut state = AppState::default();
        for c in "hello world test".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(crate::Event::CursorStart);
        state.update(crate::Event::CursorWordRight);
        // Should skip "hello" and spaces, land at start of "world"
        assert_eq!(state.input.cursor_pos, 6);
    }

    #[test]
    fn word_right_from_middle_of_word() {
        let mut state = AppState::default();
        for c in "hello world".chars() {
            state.update(crate::Event::Input(c));
        }
        state.input.cursor_pos = 2; // middle of "hello"
        state.update(crate::Event::CursorWordRight);
        // Should skip rest of "hello" and spaces, land at start of "world"
        assert_eq!(state.input.cursor_pos, 6);
    }

    #[test]
    fn word_right_at_end_does_nothing() {
        let mut state = AppState::default();
        for c in "hi".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(crate::Event::CursorEnd);
        state.update(crate::Event::CursorWordRight);
        assert_eq!(state.input.cursor_pos, 2);
    }

    #[test]
    fn word_left_multiple_times() {
        let mut state = AppState::default();
        for c in "one two three".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(crate::Event::CursorEnd);
        state.update(crate::Event::CursorWordLeft);
        assert_eq!(state.input.cursor_pos, 8); // start of "three"
        state.update(crate::Event::CursorWordLeft);
        assert_eq!(state.input.cursor_pos, 4); // start of "two"
        state.update(crate::Event::CursorWordLeft);
        assert_eq!(state.input.cursor_pos, 0); // start of "one"
    }

    #[test]
    fn word_nav_with_single_word() {
        let mut state = AppState::default();
        for c in "hello".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(crate::Event::CursorEnd);
        state.update(crate::Event::CursorWordLeft);
        assert_eq!(state.input.cursor_pos, 0);
    }
}
