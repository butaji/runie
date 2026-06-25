//! Tests for input history while in vim_nav mode.

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::Event;

    /// Regression test: HistoryPrev/HistoryNext must NOT be consumed by vim_nav
    /// handler. They should fall through to normal history navigation dispatch.
    #[test]
    fn history_events_not_consumed_in_vim_nav() {
        let mut state = AppState::default();

        // Populate input history with two submissions
        state.update(crate::Event::Input('a'));
        state.update(Event::submit());
        state.update(crate::Event::Input('b'));
        state.update(Event::submit());

        // Verify history works without vim_nav (baseline)
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "b");
        assert!(!state.view.vim_nav_mode);

        // Clear input and reset history navigation
        state.update(Event::submit());
        assert_eq!(state.input.input.len(), 0);

        // Enter vim_nav mode
        state.update(crate::Event::DialogBack);
        assert!(state.view.vim_nav_mode);

        // HistoryPrev while in vim_nav — must recall "b"
        state.update(crate::Event::HistoryPrev);
        assert_eq!(
            state.input.input, "b",
            "HistoryPrev must work in vim_nav mode (not consumed by vim nav handler)"
        );

        // HistoryPrev again — should recall "a"
        state.update(crate::Event::HistoryPrev);
        assert_eq!(
            state.input.input, "a",
            "second HistoryPrev must work in vim_nav mode"
        );

        // HistoryNext — should return to "b"
        state.update(crate::Event::HistoryNext);
        assert_eq!(
            state.input.input, "b",
            "HistoryNext must work in vim_nav mode"
        );
    }
}
