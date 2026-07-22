//! Tests for input history while in vim_nav mode.

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::Event;

    /// HistoryPrev/HistoryNext navigate the input history when the input box is
    /// active, but they are consumed by vim nav mode to move the feed selection.
    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn history_events_navigate_feed_in_vim_nav() {
        let mut state = AppState::default();

        // Populate input history with two submissions.
        state.update(crate::Event::Input('a'));
        state.update(Event::submit());
        state.update(crate::Event::Input('b'));
        state.update(Event::submit());

        // Baseline: HistoryPrev recalls history when not in nav mode (from an
        // empty input box, per grok parity).
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "b");
        assert!(!state.view.vim_nav_mode);

        // Clear the input box before entering nav mode.
        state.input.input.clear();
        state.input.cursor_pos = 0;
        state.input.history_pos = None;

        // Enter vim_nav mode.
        state.update(crate::Event::DialogBack);
        assert!(state.view.vim_nav_mode);

        // HistoryPrev while in vim_nav must be consumed for feed navigation,
        // not inserted into the input box.
        state.update(crate::Event::HistoryPrev);
        assert_eq!(
            state.input.input, "",
            "HistoryPrev in nav mode must not mutate the input box"
        );
        assert!(state.view.vim_nav_mode);

        // Repeated HistoryPrev keeps navigating the feed.
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "");
        assert!(state.view.vim_nav_mode);

        // HistoryNext moves back toward newer posts.
        state.update(crate::Event::HistoryNext);
        assert_eq!(state.input.input, "");
        assert!(state.view.vim_nav_mode);

        // Exit nav mode and verify history recall still works (empty box).
        state.update(crate::Event::DialogBack);
        assert!(!state.view.vim_nav_mode);
        state.update(crate::Event::HistoryPrev);
        assert_eq!(state.input.input, "b");
    }
}
