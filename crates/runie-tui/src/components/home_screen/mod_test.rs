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

        // home_screen_select now clears messages instead of adding a system message
        // We just verify the selection was processed (mode changed, home screen hidden)
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(!state.home_screen.is_visible());
    }

    #[test]
    fn test_new_worktree_clears_messages() {
        let mut state = create_test_state();
        state.messages.push(MessageItem::System {
            text: "Old message".to_string(),
        });
        state.home_screen.selected = 0; // "New worktree"
        home_screen_select(&mut state);

        // Messages should be cleared
        assert_eq!(state.messages.len(), 0, "Messages should be cleared after new worktree");
    }

    #[test]
    fn test_resume_session_message() {
        let mut state = create_test_state();
        state.home_screen.selected = 1; // "Resume session"
        home_screen_select(&mut state);

        // home_screen_select clears messages and switches to Chat mode
        assert_eq!(state.mode, TuiMode::Chat);
        assert!(!state.home_screen.is_visible());
    }

    #[test]
    fn test_resume_session_clears_messages() {
        let mut state = create_test_state();
        state.messages.push(MessageItem::System {
            text: "Old message".to_string(),
        });
        state.home_screen.selected = 1; // "Resume session"
        home_screen_select(&mut state);

        // Messages should be cleared
        assert_eq!(state.messages.len(), 0, "Messages should be cleared after resume session");
    }

    #[test]
    fn test_messages_not_accumulated() {
        let mut state = create_test_state();
        state.home_screen.selected = 0; // "New worktree"
        home_screen_select(&mut state);

        // Select again
        state.home_screen.selected = 0;
        home_screen_select(&mut state);

        // home_screen_select clears messages, doesn't add any
        assert_eq!(
            state.messages.len(),
            0,
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
