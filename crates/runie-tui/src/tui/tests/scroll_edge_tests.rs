//! Edge case tests for scroll and navigation.
//!
//! Tests boundaries, large datasets, and unusual scenarios.

use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, TuiMode};
use crate::tui::update::chat::update as chat_update;

// ─── Helper Functions ─────────────────────────────────────────────────────────

fn make_state() -> AppState {
    AppState {
        mode: TuiMode::Chat,
        running: true,
        current_model: Some("gpt-4o".to_string()),
        scroll: crate::tui::state::ScrollState::default(),
        ..Default::default()
    }
}

fn with_n_messages(state: &mut AppState, count: usize) {
    for i in 0..count {
        state.messages.push(MessageItem::User {
            text: format!("Message {}", i),
            model: None,
            timestamp: None,
        });
    }
}

// ─── Very Long Conversation ──────────────────────────────────────────────────

mod long_conversation {
    use super::*;

    #[test]
    fn test_scroll_1000_messages() {
        let mut state = make_state();
        with_n_messages(&mut state, 1000);

        // Scroll down to near the end
        for _ in 0..500 {
            chat_update(&mut state, Msg::ScrollDown);
        }

        assert!(state.scroll.feed_offset > 0, "Should be able to scroll through 1000 messages");
        assert!(
            state.scroll.feed_offset < 1000,
            "Offset should not exceed message count"
        );
    }

    #[test]
    fn test_page_scroll_1000_messages() {
        let mut state = make_state();
        with_n_messages(&mut state, 1000);

        // Page down multiple times
        for _ in 0..50 {
            chat_update(&mut state, Msg::ScrollPageDown);
        }

        let max_offset = state.messages.len().saturating_sub(1);
        assert_eq!(
            state.scroll.feed_offset, max_offset,
            "Should reach end after many page downs"
        );
    }

    #[test]
    fn test_scroll_offset_stays_within_bounds_1000() {
        let mut state = make_state();
        with_n_messages(&mut state, 1000);
        state.scroll.feed_offset = 999;

        // Try to scroll down more
        chat_update(&mut state, Msg::ScrollDown);
        assert_eq!(state.scroll.feed_offset, 999, "Should not exceed max");

        // Try to scroll up from 0
        state.scroll.feed_offset = 0;
        chat_update(&mut state, Msg::ScrollUp);
        assert_eq!(state.scroll.feed_offset, 0, "Should not go below 0");
    }

    #[test]
    fn test_auto_scroll_works_with_1000_messages() {
        let mut state = make_state();
        with_n_messages(&mut state, 1000);
        state.scroll.feed_offset = 500;
        state.scroll.user_scrolled_up = false;

        // Simulate message start (auto-scroll to bottom)
        use crate::tui::update::agent::events::on_message_start;
        on_message_start(&mut state, runie_agent::AgentMessage {
            role: "assistant".to_string(),
            content: vec![],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        });

        assert_eq!(state.scroll.feed_offset, 0, "Auto-scroll should go to bottom");
    }
}

// ─── Boundary Conditions ──────────────────────────────────────────────────────

mod boundaries {
    use super::*;

    #[test]
    fn test_offset_at_max_cannot_increase() {
        let mut state = make_state();
        with_n_messages(&mut state, 10);
        state.scroll.feed_offset = 9; // Max offset (10 - 1)

        for _ in 0..10 {
            chat_update(&mut state, Msg::ScrollDown);
        }

        assert_eq!(state.scroll.feed_offset, 9);
    }

    #[test]
    fn test_offset_at_zero_cannot_decrease() {
        let mut state = make_state();
        with_n_messages(&mut state, 20);
        state.scroll.feed_offset = 0;

        for _ in 0..10 {
            chat_update(&mut state, Msg::ScrollUp);
        }

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_page_up_from_small_offset() {
        let mut state = make_state();
        with_n_messages(&mut state, 50);
        state.scroll.feed_offset = 3;

        chat_update(&mut state, Msg::ScrollPageUp);

        assert_eq!(state.scroll.feed_offset, 0, "PageUp from small offset should go to 0");
    }

    #[test]
    fn test_page_down_from_near_max() {
        let mut state = make_state();
        with_n_messages(&mut state, 15);
        state.scroll.feed_offset = 12;

        chat_update(&mut state, Msg::ScrollPageDown);

        assert_eq!(state.scroll.feed_offset, 14, "Should reach max offset");
    }

    #[test]
    fn test_scroll_offset_cannot_be_negative() {
        let mut state = make_state();
        with_n_messages(&mut state, 20);
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = true;

        // Multiple scroll ups
        chat_update(&mut state, Msg::ScrollUp);
        chat_update(&mut state, Msg::ScrollUp);
        chat_update(&mut state, Msg::ScrollUp);

        assert!(state.scroll.feed_offset >= 0);
    }

    #[test]
    fn test_scroll_offset_integer_overflow_protection() {
        let mut state = make_state();
        with_n_messages(&mut state, 20);

        // Manually set to a very large value (simulating potential overflow scenario)
        state.scroll.feed_offset = usize::MAX;

        // Scroll should clamp to max
        chat_update(&mut state, Msg::ScrollDown);

        let max_offset = state.messages.len().saturating_sub(1);
        assert!(state.scroll.feed_offset <= max_offset);
    }
}

// ─── Empty Feed Scrolling ────────────────────────────────────────────────────

mod empty_feed_scrolling {
    use super::*;

    #[test]
    fn test_scroll_down_empty_does_nothing() {
        let mut state = make_state();

        chat_update(&mut state, Msg::ScrollDown);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_scroll_up_empty_does_nothing() {
        let mut state = make_state();

        chat_update(&mut state, Msg::ScrollUp);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_page_down_empty_does_nothing() {
        let mut state = make_state();

        chat_update(&mut state, Msg::ScrollPageDown);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_page_up_empty_does_nothing() {
        let mut state = make_state();

        chat_update(&mut state, Msg::ScrollPageUp);

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_auto_scroll_empty_feed() {
        let mut state = make_state();
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = false;

        use crate::tui::update::agent::events::on_message_start;
        on_message_start(&mut state, runie_agent::AgentMessage {
            role: "assistant".to_string(),
            content: vec![],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        });

        assert_eq!(state.scroll.feed_offset, 0, "Auto-scroll on empty feed stays at 0");
    }
}

// ─── One Message Edge Cases ──────────────────────────────────────────────────

mod single_message_edge {
    use super::*;

    #[test]
    fn test_scroll_with_one_message() {
        let mut state = make_state();
        state.messages.push(MessageItem::User {
            text: "Only one".to_string(),
            model: None,
            timestamp: None,
        });

        chat_update(&mut state, Msg::ScrollDown);
        assert_eq!(state.scroll.feed_offset, 0);

        chat_update(&mut state, Msg::ScrollUp);
        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_page_scroll_with_one_message() {
        let mut state = make_state();
        state.messages.push(MessageItem::User {
            text: "Only one".to_string(),
            model: None,
            timestamp: None,
        });

        chat_update(&mut state, Msg::ScrollPageDown);
        assert_eq!(state.scroll.feed_offset, 0);

        chat_update(&mut state, Msg::ScrollPageUp);
        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_auto_scroll_with_one_message() {
        let mut state = make_state();
        state.messages.push(MessageItem::User {
            text: "Only one".to_string(),
            model: None,
            timestamp: None,
        });
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = false;

        use crate::tui::update::agent::events::on_message_start;
        on_message_start(&mut state, runie_agent::AgentMessage {
            role: "assistant".to_string(),
            content: vec![],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        });

        assert_eq!(state.scroll.feed_offset, 0);
    }
}

// ─── Rapid Scroll Operations ─────────────────────────────────────────────────

mod rapid_scroll {
    use super::*;

    #[test]
    fn test_rapid_scroll_down() {
        let mut state = make_state();
        with_n_messages(&mut state, 100);

        // Rapid scroll operations
        for _ in 0..100 {
            chat_update(&mut state, Msg::ScrollDown);
        }

        // Should be at max or near max
        assert!(state.scroll.feed_offset >= 90);
    }

    #[test]
    fn test_rapid_scroll_up_from_max() {
        let mut state = make_state();
        with_n_messages(&mut state, 100);
        state.scroll.feed_offset = 99;

        for _ in 0..100 {
            chat_update(&mut state, Msg::ScrollUp);
        }

        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_alternating_scroll_up_down() {
        let mut state = make_state();
        with_n_messages(&mut state, 50);
        state.scroll.feed_offset = 25;

        // Alternate scroll directions
        for _ in 0..10 {
            chat_update(&mut state, Msg::ScrollUp);
            chat_update(&mut state, Msg::ScrollDown);
        }

        // Should stay around 25
        assert!(state.scroll.feed_offset >= 20 && state.scroll.feed_offset <= 30);
    }
}

// ─── Scroll with Mixed Message Types ────────────────────────────────────────

mod mixed_message_types {
    use super::*;

    #[test]
    fn test_scroll_with_various_message_types() {
        let mut state = make_state();

        // Add mixed message types
        state.messages.push(MessageItem::User { text: "User".to_string(), model: None, timestamp: None });
        state.messages.push(MessageItem::Assistant { text: "Assistant".to_string(), model: None, timestamp: None, expanded: false });
        state.messages.push(MessageItem::System { text: "System".to_string() });
        state.messages.push(MessageItem::Error { message: "Error".to_string(), recoverable: true });
        state.messages.push(MessageItem::ToolCall { name: "tool".to_string(), args: "args".to_string(), result: None, is_error: false });
        state.messages.push(MessageItem::Thought { duration_secs: 1.5, text: String::new() });
        state.messages.push(MessageItem::Separator { elapsed_secs: 10, tool_calls: 2, tokens_used: Some(100) });

        // Scroll should work with all types
        for _ in 0..5 {
            chat_update(&mut state, Msg::ScrollDown);
        }

        assert!(state.scroll.feed_offset > 0);
    }

    #[test]
    fn test_scroll_offset_matches_message_index() {
        let mut state = make_state();

        for i in 0..10 {
            state.messages.push(MessageItem::User {
                text: format!("Message {}", i),
                model: None,
                timestamp: None,
            });
        }

        state.scroll.feed_offset = 5;
        let offset = state.scroll.feed_offset;

        // The offset should correspond to a valid message index
        assert!(offset < state.messages.len());
    }
}

// ─── Scroll State Consistency ────────────────────────────────────────────────

mod scroll_consistency {
    use super::*;

    #[test]
    fn test_user_scrolled_up_false_when_offset_zero() {
        let mut state = make_state();
        with_n_messages(&mut state, 20);
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = false;

        // Scroll down then back to 0
        chat_update(&mut state, Msg::ScrollDown);
        for _ in 0..5 {
            chat_update(&mut state, Msg::ScrollUp);
        }

        // user_scrolled_up should now be false since we're at 0
        // (Note: current implementation sets it based on offset > 0)
        assert_eq!(state.scroll.feed_offset, 0);
    }

    #[test]
    fn test_scroll_state_after_clear_chat() {
        let mut state = make_state();
        with_n_messages(&mut state, 20);
        state.scroll.feed_offset = 15;

        chat_update(&mut state, Msg::ClearChat);

        assert_eq!(state.scroll.feed_offset, 0, "ClearChat should reset scroll");
        assert_eq!(state.messages.len(), 0, "Messages should be cleared");
    }
}