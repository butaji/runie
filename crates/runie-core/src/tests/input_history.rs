//! Tests for input history (Up/Down arrows)
//!
//! Contract (grok parity): Up/Down recall history only when the input box is
//! EMPTY (or while an unmodified recalled entry is showing). With text in the
//! box, arrows move the cursor — drafts can never be clobbered by history
//! navigation. Feed scrolling uses PgUp/PgDn and Esc nav mode.

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::Event;

    #[test]
    fn history_starts_empty() {
        let state = AppState::default();
        assert!(state.input.input.is_empty());
    }

    #[test]
    fn submit_adds_to_history() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('h'));
        state.update(crate::Event::Input('i'));
        state.update(Event::submit());
        assert!(state.input.input.is_empty());
        // History should have "hi"
    }

    #[test]
    fn up_arrow_recalls_previous_input() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('h'));
        state.update(crate::Event::Input('i'));
        state.update(Event::submit());
        // Up on an EMPTY input recalls the latest history entry.
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "hi");
    }

    #[test]
    fn down_arrow_returns_to_empty() {
        let mut state = AppState::default();
        state.update(crate::Event::Input('h'));
        state.update(crate::Event::Input('i'));
        state.update(Event::submit());
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "hi");
        state.update(crate::Event::HistoryNext);
        assert!(state.input.input.is_empty());
    }

    #[test]
    fn history_navigates_multiple_items() {
        let mut state = AppState::default();
        // Submit "first"
        for c in "first".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(Event::submit());
        // Submit "second"
        for c in "second".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(Event::submit());
        // Submit "third"
        for c in "third".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(Event::submit());

        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "third");
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "second");
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "first");
        // At beginning, stays at first
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "first");
    }

    #[test]
    fn history_next_at_end_is_empty() {
        let mut state = AppState::default();
        for c in "test".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(Event::submit());
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "test");
        state.update(crate::Event::HistoryNext);
        assert!(state.input.input.is_empty());
        // Stays empty (further Down on empty input flashes, does not scroll)
        state.update(crate::Event::HistoryNext);
        assert!(state.input.input.is_empty());
        assert_eq!(state.view().scroll, 0);
    }

    /// Editing a recalled entry exits history mode: the edit can never be
    /// clobbered by a subsequent Up — with text in the box, arrows move the
    /// cursor (grok parity).
    #[test]
    fn editing_resets_history_nav() {
        let mut state = AppState::default();
        for c in "hello".chars() {
            state.update(crate::Event::Input(c));
        }
        state.update(Event::submit());
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "hello");
        // Type something new
        state.update(crate::Event::Input('x'));
        assert_eq!(state.input.input, "hellox");
        // Up now moves the cursor to the start instead of recalling history;
        // Down moves it back to the end. The edit survives both.
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "hellox");
        assert_eq!(state.input.cursor_pos, 0);
        state.update(crate::Event::HistoryNext);
        assert_eq!(state.input.input, "hellox");
        assert_eq!(state.input.cursor_pos, "hellox".len());
    }
}
