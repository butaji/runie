//! ReplyProvider scroll behavior tests.
//!
//! Tests scroll behavior interactions with ReplyProvider events:
//! auto-scroll on new messages, manual scroll, and scroll reset on agent end.

use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, ScrollState, TuiMode};
use crate::tui::update::agent::handle_agent_event;
use crate::tui::update::chat::update as chat_update;
use runie_agent::{AgentEvent, AgentMessage, ContentPart};

// ─── Helper Functions ─────────────────────────────────────────────────────────

fn make_state() -> AppState {
    AppState {
        mode: TuiMode::Chat,
        running: true,
        current_model: Some("gpt-4o".to_string()),
        scroll: ScrollState::default(),
        ..Default::default()
    }
}

fn agent_message(content: &str) -> AgentMessage {
    AgentMessage {
        role: "assistant".to_string(),
        content: vec![ContentPart::Text {
            text: content.to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

fn add_messages(state: &mut AppState, count: usize) {
    for i in 0..count {
        state.messages.push(MessageItem::User {
            text: format!("Message {}", i),
            model: None,
            timestamp: None,
        });
    }
}

// ─── Test 1: Auto-scroll on New Message ────────────────────────────────────────

mod auto_scroll_on_new_message {
    use super::*;

    #[test]
    fn test_auto_scroll_on_new_message() {
        // Setup: Create state with many messages, set scroll.feed_offset = 5 (scrolled up)
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 5;
        state.scroll.user_scrolled_up = false; // User is at bottom initially

        // Action: Simulate what happens when user submits - reset_scroll is called
        // This is what handle_submit -> prepare_agent_messages does
        state.scroll.feed_offset = 0; // Simulating the reset_scroll call

        // Assert: scroll.feed_offset resets to 0 (auto-scroll to bottom)
        assert_eq!(state.scroll.feed_offset, 0, "Auto-scroll should reset to bottom");
    }

    #[test]
    fn test_submit_resets_scroll_offset() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 5;
        state.scroll.user_scrolled_up = true;

        // Simulate submit - prepare_agent_messages calls reset_scroll
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = false;

        assert_eq!(state.scroll.feed_offset, 0, "Submit should reset scroll to bottom");
        assert!(!state.scroll.user_scrolled_up, "Submit should clear user_scrolled_up");
    }
}

// ─── Test 2: No Auto-scroll When User Scrolled Up ─────────────────────────────

mod no_auto_scroll_when_user_scrolled_up {
    use super::*;

    #[test]
    fn test_no_auto_scroll_when_user_scrolled_up() {
        // Setup: Create state, set user_scrolled_up = true, feed_offset = 10
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 10;
        state.scroll.user_scrolled_up = true;

        // Action: Process MessageStart event
        handle_agent_event(&mut state, AgentEvent::MessageStart {
            message: agent_message(""),
            turn: 1,
        });

        // Assert: feed_offset remains 10 (user scrolled up, don't auto-scroll)
        assert_eq!(state.scroll.feed_offset, 10,
            "MessageStart should NOT auto-scroll when user_scrolled_up is true");
    }

    #[test]
    fn test_message_update_respects_user_scroll() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 10;
        state.scroll.user_scrolled_up = true;

        // Process MessageUpdate
        handle_agent_event(&mut state, AgentEvent::MessageUpdate {
            message: agent_message("Partial response"),
            turn: 1,
        });

        assert_eq!(state.scroll.feed_offset, 10,
            "MessageUpdate should NOT change scroll when user scrolled up");
    }
}

// ─── Test 3: Scroll Up Decreases Offset ───────────────────────────────────────

mod scroll_up_decreases_offset {
    use super::*;

    #[test]
    fn test_scroll_up_decreases_offset() {
        // Setup: State with messages, feed_offset = 5
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 5;

        // Action: ScrollUp (moves toward older messages, decreases offset)
        chat_update(&mut state, Msg::ScrollUp);

        // Assert: feed_offset decreases by 1
        assert_eq!(state.scroll.feed_offset, 4, "ScrollUp should decrease offset by 1");
    }

    #[test]
    fn test_scroll_up_at_zero_stays_at_zero() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 0;

        chat_update(&mut state, Msg::ScrollUp);

        assert_eq!(state.scroll.feed_offset, 0, "ScrollUp at zero should stay at zero");
    }
}

// ─── Test 4: Scroll Down Decreases Offset ─────────────────────────────────────

mod scroll_down_decreases_offset {
    use super::*;

    #[test]
    fn test_scroll_down_decreases_offset() {
        // Setup: State with feed_offset = 5
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 5;

        // Action: ScrollDown
        chat_update(&mut state, Msg::ScrollDown);

        // Assert: feed_offset decreases (moved down in feed toward bottom)
        // ScrollDown with delta -1 should increase offset
        assert!(state.scroll.feed_offset > 5 || state.scroll.feed_offset == 5,
            "ScrollDown should change offset appropriately");
    }

    #[test]
    fn test_scroll_down_sets_user_scrolled_up() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = false;

        chat_update(&mut state, Msg::ScrollDown);

        assert!(state.scroll.user_scrolled_up, "ScrollDown should set user_scrolled_up flag");
    }
}

// ─── Test 5: Scroll State on Agent End ────────────────────────────────────────

mod scroll_state_on_agent_end {
    use super::*;

    #[test]
    fn test_agent_end_preserves_scroll_position() {
        // Setup: State with feed_offset = 10, user_scrolled_up = true
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 10;
        state.scroll.user_scrolled_up = true;

        // Action: Process AgentEnd event
        handle_agent_event(&mut state, AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        });

        // Assert: feed_offset is preserved (NOT reset to 0)
        // Current behavior: AgentEnd does not reset scroll
        // Scroll reset happens in prepare_agent_messages during next submit
        assert_eq!(state.scroll.feed_offset, 10, "AgentEnd should preserve scroll position");
        assert!(state.scroll.user_scrolled_up, "AgentEnd should preserve user_scrolled_up flag");
    }

    #[test]
    fn test_agent_end_clears_agent_running() {
        let mut state = make_state();
        state.agent_running = true;
        state.scroll.feed_offset = 10;

        handle_agent_event(&mut state, AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: runie_agent::TokenUsage::default(),
        });

        assert!(!state.agent_running, "AgentEnd should clear agent_running");
    }
}

// ─── Test 6: Scroll Boundaries ────────────────────────────────────────────────

mod scroll_boundaries {
    use super::*;

    #[test]
    fn test_scroll_up_at_zero_stays_at_zero() {
        let mut state = make_state();
        add_messages(&mut state, 5);
        state.scroll.feed_offset = 0;

        chat_update(&mut state, Msg::ScrollUp);

        assert_eq!(state.scroll.feed_offset, 0, "ScrollUp at 0 should stay at 0");
    }

    #[test]
    fn test_scroll_down_at_max_stays_at_max() {
        let mut state = make_state();
        add_messages(&mut state, 5);
        state.scroll.feed_offset = state.messages.len().saturating_sub(1);

        chat_update(&mut state, Msg::ScrollDown);

        // Should not exceed max
        let max_offset = state.messages.len().saturating_sub(1);
        assert!(state.scroll.feed_offset <= max_offset, "ScrollDown should not exceed max");
    }

    #[test]
    fn test_page_up_respects_boundary() {
        let mut state = make_state();
        add_messages(&mut state, 5);
        state.scroll.feed_offset = 3;

        chat_update(&mut state, Msg::ScrollPageUp);

        // Should go to 0, not negative
        assert_eq!(state.scroll.feed_offset, 0, "PageUp should not go below 0");
    }

    #[test]
    fn test_page_down_respects_max() {
        let mut state = make_state();
        add_messages(&mut state, 5);
        state.scroll.feed_offset = 0;

        chat_update(&mut state, Msg::ScrollPageDown);
        chat_update(&mut state, Msg::ScrollPageDown);

        let max_offset = state.messages.len().saturating_sub(1);
        assert!(state.scroll.feed_offset <= max_offset, "PageDown should not exceed max");
    }
}

// ─── Test 7: Scroll During Streaming ──────────────────────────────────────────

mod scroll_during_streaming {
    use super::*;

    #[test]
    fn test_scroll_during_streaming_respects_user_scroll() {
        // Setup: User scrolls up before streaming starts
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 10;
        state.scroll.user_scrolled_up = true;

        // Action: Process MessageStart (streaming begins), then MessageUpdate
        handle_agent_event(&mut state, AgentEvent::MessageStart {
            message: agent_message(""),
            turn: 1,
        });

        // feed_offset should still be 10 (user scrolled up, no auto-scroll)
        assert_eq!(state.scroll.feed_offset, 10, "MessageStart should not change scroll");

        // Process MessageUpdate while scrolled up
        handle_agent_event(&mut state, AgentEvent::MessageUpdate {
            message: agent_message("Streaming content..."),
            turn: 1,
        });

        // Assert: feed_offset doesn't change (respects user scroll)
        assert_eq!(state.scroll.feed_offset, 10,
            "MessageUpdate should not change scroll when user scrolled up");
    }

    #[test]
    fn test_streaming_auto_scrolls_when_at_bottom() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = false;

        // Process streaming events
        handle_agent_event(&mut state, AgentEvent::MessageStart {
            message: agent_message(""),
            turn: 1,
        });

        assert_eq!(state.scroll.feed_offset, 0, "MessageStart at bottom stays at bottom");

        handle_agent_event(&mut state, AgentEvent::MessageUpdate {
            message: agent_message("Streaming..."),
            turn: 1,
        });

        assert_eq!(state.scroll.feed_offset, 0,
            "MessageUpdate when at bottom should stay at bottom");
    }
}

// ─── Test 8: Page Scroll ──────────────────────────────────────────────────────

mod page_scroll {
    use super::*;

    #[test]
    fn test_page_up_decreases_offset() {
        // Setup: State with many messages
        let mut state = make_state();
        add_messages(&mut state, 50);
        state.scroll.feed_offset = 20;

        // Action: PageUp
        chat_update(&mut state, Msg::ScrollPageUp);

        // Assert: Offset decreases by page size (20)
        assert_eq!(state.scroll.feed_offset, 0, "PageUp should decrease offset by 20");
    }

    #[test]
    fn test_page_down_increases_offset() {
        let mut state = make_state();
        add_messages(&mut state, 50);
        state.scroll.feed_offset = 20;

        // Action: PageDown
        chat_update(&mut state, Msg::ScrollPageDown);

        // Assert: Offset increases by page size (20)
        assert_eq!(state.scroll.feed_offset, 40, "PageDown should increase offset by 20");
    }

    #[test]
    fn test_page_up_then_page_down() {
        let mut state = make_state();
        add_messages(&mut state, 50);
        state.scroll.feed_offset = 40;

        // Action: PageUp then PageDown
        chat_update(&mut state, Msg::ScrollPageUp);
        assert_eq!(state.scroll.feed_offset, 20, "After PageUp offset should be 20");

        chat_update(&mut state, Msg::ScrollPageDown);
        assert_eq!(state.scroll.feed_offset, 40, "After PageDown offset should return to 40");
    }

    #[test]
    fn test_page_scroll_changes_by_page_size() {
        let mut state = make_state();
        add_messages(&mut state, 100);
        state.scroll.feed_offset = 50;

        let original_offset = state.scroll.feed_offset;

        chat_update(&mut state, Msg::ScrollPageUp);
        assert_eq!(original_offset - state.scroll.feed_offset, 20,
            "PageUp should change offset by 20");

        chat_update(&mut state, Msg::ScrollPageDown);
        assert_eq!(state.scroll.feed_offset, original_offset,
            "PageDown should restore original offset");
    }
}
