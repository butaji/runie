//! Scroll navigation tests.
//!
//! Tests basic scroll up/down, page up/down, and offset management.

use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, TuiMode};
use crate::tui::update::chat::update as chat_update;

/// Creates a basic AppState for testing.
fn make_state() -> AppState {
    AppState {
        mode: TuiMode::Chat,
        running: true,
        current_model: Some("gpt-4o".to_string()),
        scroll: crate::tui::state::ScrollState::default(),
        ..Default::default()
    }
}

/// Adds N user messages to the state for scroll testing.
fn with_messages(state: &mut AppState, count: usize) {
    for i in 0..count {
        state.messages.push(MessageItem::User {
            text: format!("Message {}", i),
            model: None,
            timestamp: None,
        });
    }
}

// ─── Basic Scroll Up/Down ──────────────────────────────────────────────────────

mod scroll_basic {
    use super::*;

    #[test]
    fn test_scroll_down_increases_offset() {
        let mut state = make_state();
        with_messages(&mut state, 20);

        // Scroll down should increase offset
        chat_update(&mut state, Msg::ScrollDown);
        assert!(state.scroll.feed_offset > 0, "ScrollDown should increase offset");
    }

    #[test]
    fn test_scroll_up_decreases_offset() {
        let mut state = make_state();
        with_messages(&mut state, 20);
        state.scroll.feed_offset = 5;

        chat_update(&mut state, Msg::ScrollUp);
        assert_eq!(state.scroll.feed_offset, 4, "ScrollUp should decrease offset");
    }

    #[test]
    fn test_scroll_up_cannot_go_below_zero() {
        let mut state = make_state();
        with_messages(&mut state, 20);
        state.scroll.feed_offset = 0;

        chat_update(&mut state, Msg::ScrollUp);
        assert_eq!(state.scroll.feed_offset, 0, "Scroll should not go below 0");
    }

    #[test]
    fn test_scroll_down_cannot_exceed_max() {
        let mut state = make_state();
        with_messages(&mut state, 10);

        // Scroll to max
        for _ in 0..100 {
            chat_update(&mut state, Msg::ScrollDown);
        }

        let max_offset = state.messages.len().saturating_sub(1);
        assert_eq!(
            state.scroll.feed_offset, max_offset,
            "Scroll should not exceed max offset"
        );
    }
}

// ─── Page Up/Down ─────────────────────────────────────────────────────────────

mod scroll_page {
    use super::*;

    #[test]
    fn test_page_down_increases_offset_by_page_size() {
        let mut state = make_state();
        with_messages(&mut state, 50);
        state.scroll.feed_offset = 0;

        // PAGE_SIZE is 20 in handle_scroll_msg; see crates/runie-tui/src/tui/update/chat.rs
        chat_update(&mut state, Msg::ScrollPageDown);
        assert_eq!(state.scroll.feed_offset, 20, "PageDown should increase offset by PAGE_SIZE");
    }

    #[test]
    fn test_page_up_decreases_offset_by_page_size() {
        let mut state = make_state();
        with_messages(&mut state, 50);
        state.scroll.feed_offset = 40;

        chat_update(&mut state, Msg::ScrollPageUp);
        assert_eq!(state.scroll.feed_offset, 20, "PageUp should decrease offset by PAGE_SIZE");
    }

    #[test]
    fn test_page_up_cannot_go_below_zero() {
        let mut state = make_state();
        with_messages(&mut state, 50);
        state.scroll.feed_offset = 5;

        chat_update(&mut state, Msg::ScrollPageUp);
        assert_eq!(state.scroll.feed_offset, 0, "PageUp should not go below 0");
    }

    #[test]
    fn test_page_down_respects_max() {
        let mut state = make_state();
        with_messages(&mut state, 15);
        state.scroll.feed_offset = 10;

        chat_update(&mut state, Msg::ScrollPageDown);
        let max_offset = state.messages.len().saturating_sub(1);
        assert_eq!(
            state.scroll.feed_offset, max_offset,
            "PageDown should not exceed max offset"
        );
    }
}

// ─── user_scrolled_up Flag ───────────────────────────────────────────────────

mod user_scrolled_up_flag {
    use super::*;

    #[test]
    fn test_scroll_down_sets_user_scrolled_up() {
        let mut state = make_state();
        with_messages(&mut state, 20);
        state.scroll.user_scrolled_up = false;

        chat_update(&mut state, Msg::ScrollDown);

        assert!(state.scroll.user_scrolled_up, "ScrollDown should set user_scrolled_up");
    }

    #[test]
    fn test_scroll_up_clears_user_scrolled_up_when_at_zero() {
        let mut state = make_state();
        with_messages(&mut state, 20);
        state.scroll.feed_offset = 1;
        state.scroll.user_scrolled_up = true;

        chat_update(&mut state, Msg::ScrollUp);

        // Scrolling up from offset=1 clamps to 0, at which point
        // user_scrolled_up flips to false (no more history above).
        assert_eq!(state.scroll.feed_offset, 0);
        assert!(!state.scroll.user_scrolled_up);
    }

    #[test]
    fn test_user_scrolled_up_true_means_user_explicitly_scrolled() {
        let mut state = make_state();
        with_messages(&mut state, 20);
        state.scroll.feed_offset = 10;
        state.scroll.user_scrolled_up = true;

        // Verify flag is preserved during scroll
        chat_update(&mut state, Msg::ScrollDown);
        assert!(state.scroll.user_scrolled_up);
    }
}

// ─── Scroll Reset on Submit ──────────────────────────────────────────────────

mod scroll_reset_on_submit {
    use super::*;

    #[test]
    fn test_scroll_offset_reset_on_submit() {
        let mut state = make_state();
        with_messages(&mut state, 20);
        state.scroll.feed_offset = 15;
        state.scroll.user_scrolled_up = true;

        // Set up textarea for submit
        state.textarea = ratatui_textarea::TextArea::new(vec!["Hello".to_string()]);

        chat_update(&mut state, Msg::Submit);

        assert_eq!(state.scroll.feed_offset, 0, "Submit should reset scroll offset");
    }

    #[test]
    fn test_user_scrolled_up_reset_on_submit() {
        let mut state = make_state();
        with_messages(&mut state, 20);
        state.scroll.feed_offset = 15;
        state.scroll.user_scrolled_up = true;

        state.textarea = ratatui_textarea::TextArea::new(vec!["Hello".to_string()]);

        chat_update(&mut state, Msg::Submit);

        assert!(!state.scroll.user_scrolled_up, "Submit should reset user_scrolled_up flag");
    }
}

// ─── Scroll with Empty Feed ──────────────────────────────────────────────────

mod scroll_empty_feed {
    use super::*;

    #[test]
    fn test_scroll_down_with_empty_feed_does_not_panic() {
        let mut state = make_state();
        // No messages

        // Should not panic
        chat_update(&mut state, Msg::ScrollDown);

        assert_eq!(state.scroll.feed_offset, 0, "Scroll should stay at 0 with empty feed");
    }

    #[test]
    fn test_scroll_up_with_empty_feed_does_not_panic() {
        let mut state = make_state();

        chat_update(&mut state, Msg::ScrollUp);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_page_down_with_empty_feed_does_not_panic() {
        let mut state = make_state();

        chat_update(&mut state, Msg::ScrollPageDown);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_page_up_with_empty_feed_does_not_panic() {
        let mut state = make_state();

        chat_update(&mut state, Msg::ScrollPageUp);

        assert_eq!(state.scroll.feed_offset, 0);
    }
}

// ─── Scroll with Single Message ──────────────────────────────────────────────

mod scroll_single_message {
    use super::*;

    #[test]
    fn test_scroll_with_single_message() {
        let mut state = make_state();
        state.messages.push(MessageItem::User {
            text: "Only message".to_string(),
            model: None,
            timestamp: None,
        });

        chat_update(&mut state, Msg::ScrollDown);

        // With only 1 message, offset should stay at 0 (1 - 1 = 0)
        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_scroll_up_with_single_message() {
        let mut state = make_state();
        state.messages.push(MessageItem::User {
            text: "Only message".to_string(),
            model: None,
            timestamp: None,
        });
        state.scroll.feed_offset = 0;

        chat_update(&mut state, Msg::ScrollUp);

        assert_eq!(state.scroll.feed_offset, 0);
    }
}

// ─── Scroll Reset on Slash Commands ─────────────────────────────────────────

mod scroll_reset_on_slash {
    use super::*;
    use crate::tui::update::slash::handle_slash;
    use runie_core::slash_command::SlashCommand;

    #[test]
    fn test_new_resets_scroll_offset() {
        let mut state = make_state();
        state.scroll.feed_offset = 50;
        state.scroll.user_scrolled_up = true;

        handle_slash(&mut state, SlashCommand::New);

        assert_eq!(state.scroll.feed_offset, 0);
        assert!(!state.scroll.user_scrolled_up);
    }

    #[test]
    fn test_clear_resets_scroll_offset() {
        let mut state = make_state();
        state.scroll.feed_offset = 50;
        state.scroll.user_scrolled_up = true;

        handle_slash(&mut state, SlashCommand::Clear);

        assert_eq!(state.scroll.feed_offset, 0);
        assert!(!state.scroll.user_scrolled_up);
    }
}

// ─── Scroll Preserved Across Mode Switches ──────────────────────────────────

mod scroll_preserved_mode_switch {
    use super::*;

    #[test]
    fn test_scroll_offset_preserved_on_mode_switch() {
        let mut state = make_state();
        with_messages(&mut state, 50);
        state.scroll.feed_offset = 30;
        state.scroll.user_scrolled_up = true;

        // Switch to a different mode and back
        let original_offset = state.scroll.feed_offset;
        let original_flag = state.scroll.user_scrolled_up;

        state.mode = TuiMode::SessionTree;
        assert_eq!(state.scroll.feed_offset, original_offset, "Offset preserved during mode switch");

        state.mode = TuiMode::Chat;
        assert_eq!(state.scroll.feed_offset, original_offset, "Offset preserved after returning to Chat");
        assert_eq!(state.scroll.user_scrolled_up, original_flag);
    }

    #[test]
    fn test_scroll_offset_preserved_on_tree_toggle() {
        let mut state = make_state();
        with_messages(&mut state, 50);
        state.scroll.feed_offset = 25;

        // Toggle tree mode on
        state.session_tree.visible = true;
        state.mode = TuiMode::SessionTree;

        // Toggle tree mode off
        state.session_tree.visible = false;
        state.mode = TuiMode::Chat;

        assert_eq!(state.scroll.feed_offset, 25, "Scroll offset preserved across tree toggle");
    }
}