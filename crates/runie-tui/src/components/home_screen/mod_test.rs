use crate::components::{HomeScreen, MessageItem};
use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::chat::modal::home_screen_select;

fn create_test_state() -> AppState {
    let mut state = AppState::default();
    state.mode = TuiMode::HomeScreen;
    state.home_screen = HomeScreen::new();
    state.messages.clear();
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_worktree_message() {
        let mut state = create_test_state();
        state.home_screen.selected = 0; // "New worktree"
        home_screen_select(&mut state);

        let last_msg = state.messages.last();
        assert!(
            matches!(
                last_msg,
                Some(MessageItem::System { text }) if text == "New session started"
            ),
            "Expected 'New session started', got {:?}",
            last_msg
        );
    }

    #[test]
    fn test_new_worktree_clears_messages() {
        let mut state = create_test_state();
        state.messages.push(MessageItem::System {
            text: "Old message".to_string(),
        });
        state.home_screen.selected = 0; // "New worktree"
        home_screen_select(&mut state);

        // Should have exactly 1 message (cleared + new message)
        assert_eq!(state.messages.len(), 1);
        assert!(
            matches!(
                state.messages.last(),
                Some(MessageItem::System { text }) if text == "New session started"
            ),
            "Expected only new message after clear"
        );
    }

    #[test]
    fn test_resume_session_message() {
        let mut state = create_test_state();
        state.home_screen.selected = 1; // "Resume session"
        home_screen_select(&mut state);

        let last_msg = state.messages.last();
        assert!(
            matches!(
                last_msg,
                Some(MessageItem::System { text }) if text == "Resuming last session"
            ),
            "Expected 'Resuming last session', got {:?}",
            last_msg
        );
    }

    #[test]
    fn test_resume_session_clears_messages() {
        let mut state = create_test_state();
        state.messages.push(MessageItem::System {
            text: "Old message".to_string(),
        });
        state.home_screen.selected = 1; // "Resume session"
        home_screen_select(&mut state);

        // Should have exactly 1 message (cleared + new message)
        assert_eq!(state.messages.len(), 1);
        assert!(
            matches!(
                state.messages.last(),
                Some(MessageItem::System { text }) if text == "Resuming last session"
            ),
            "Expected only new message after clear"
        );
    }

    #[test]
    fn test_messages_not_accumulated() {
        let mut state = create_test_state();
        state.home_screen.selected = 0; // "New worktree"
        home_screen_select(&mut state);

        // Select again
        state.home_screen.selected = 0;
        home_screen_select(&mut state);

        // Should still have only 1 message, not 2
        assert_eq!(
            state.messages.len(),
            1,
            "Messages should not accumulate across multiple selections"
        );
    }

    #[test]
    fn test_new_worktree_switches_to_chat_mode() {
        let mut state = create_test_state();
        assert_eq!(state.mode, TuiMode::HomeScreen);
        state.home_screen.selected = 0; // "New worktree"
        home_screen_select(&mut state);

        assert_eq!(
            state.mode, TuiMode::Chat,
            "Mode should switch to Chat after selecting New worktree"
        );
    }

    #[test]
    fn test_resume_session_switches_to_chat_mode() {
        let mut state = create_test_state();
        assert_eq!(state.mode, TuiMode::HomeScreen);
        state.home_screen.selected = 1; // "Resume session"
        home_screen_select(&mut state);

        assert_eq!(
            state.mode, TuiMode::Chat,
            "Mode should switch to Chat after selecting Resume session"
        );
    }

    #[test]
    fn test_new_worktree_hides_home_screen() {
        let mut state = create_test_state();
        assert!(state.home_screen.is_visible());
        state.home_screen.selected = 0; // "New worktree"
        home_screen_select(&mut state);

        assert!(
            !state.home_screen.is_visible(),
            "Home screen should be hidden after selection"
        );
    }

    #[test]
    fn test_resume_session_hides_home_screen() {
        let mut state = create_test_state();
        assert!(state.home_screen.is_visible());
        state.home_screen.selected = 1; // "Resume session"
        home_screen_select(&mut state);

        assert!(
            !state.home_screen.is_visible(),
            "Home screen should be hidden after selection"
        );
    }
}
