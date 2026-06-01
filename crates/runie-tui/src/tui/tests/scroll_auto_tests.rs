//! Auto-scroll tests.
//!
//! Tests auto-scroll behavior on MessageStart/Update/End events
//! and user_scrolled_up interaction.

use crate::components::MessageItem;
use crate::tui::state::{AppState, ThinkingState, TuiMode};
use crate::tui::update::agent::events::{
    handle_agent_event, on_message_end, on_message_start, on_message_update,
};
use runie_agent::{AgentEvent, AgentMessage, ContentPart};

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

fn make_agent_message(content: &str) -> AgentMessage {
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

// ─── Auto-scroll on MessageStart ─────────────────────────────────────────────

mod auto_scroll_message_start {
    use super::*;

    #[test]
    fn test_message_start_resets_scroll_when_at_bottom() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = false;

        on_message_start(&mut state, make_agent_message("Hello"));

        assert_eq!(state.scroll.feed_offset, 0, "MessageStart should keep offset at 0 when at bottom");
    }

    #[test]
    fn test_message_start_resets_scroll_when_scrolled_up() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 10;
        state.scroll.user_scrolled_up = true;

        on_message_start(&mut state, make_agent_message("Hello"));

        // Auto-scroll should NOT happen when user_scrolled_up is true
        assert_eq!(state.scroll.feed_offset, 10, "MessageStart should NOT reset when user scrolled up");
    }

    #[test]
    fn test_message_start_sets_offset_to_zero_when_user_not_scrolled() {
        let mut state = make_state();
        add_messages(&mut state, 50);
        state.scroll.feed_offset = 25;
        state.scroll.user_scrolled_up = false;

        on_message_start(&mut state, make_agent_message("Auto-scroll should go to bottom"));

        assert_eq!(state.scroll.feed_offset, 0, "Should auto-scroll to bottom (offset 0)");
    }

    #[test]
    fn test_message_start_does_not_affect_user_scrolled_up_flag() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 5;
        state.scroll.user_scrolled_up = true;

        on_message_start(&mut state, make_agent_message("Test"));

        // user_scrolled_up remains true (auto-scroll doesn't change it)
        assert!(state.scroll.user_scrolled_up);
    }
}

// ─── Auto-scroll on MessageUpdate ───────────────────────────────────────────

mod auto_scroll_message_update {
    use super::*;

    #[test]
    fn test_message_update_resets_scroll_when_at_bottom() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = false;

        on_message_update(&mut state, make_agent_message("Updated content"));

        assert_eq!(state.scroll.feed_offset, 0, "MessageUpdate should keep offset at 0 when at bottom");
    }

    #[test]
    fn test_message_update_does_not_scroll_when_user_scrolled_up() {
        let mut state = make_state();
        add_messages(&mut state, 50);
        state.scroll.feed_offset = 30;
        state.scroll.user_scrolled_up = true;

        on_message_update(&mut state, make_agent_message("Partial update"));

        assert_eq!(
            state.scroll.feed_offset, 30,
            "MessageUpdate should NOT reset scroll when user scrolled up"
        );
    }

    #[test]
    fn test_message_update_multiple_times_preserves_user_scroll() {
        let mut state = make_state();
        add_messages(&mut state, 50);
        state.scroll.feed_offset = 40;
        state.scroll.user_scrolled_up = true;

        // Multiple updates
        on_message_update(&mut state, make_agent_message("Update 1"));
        on_message_update(&mut state, make_agent_message("Update 2"));
        on_message_update(&mut state, make_agent_message("Update 3"));

        assert_eq!(
            state.scroll.feed_offset, 40,
            "Multiple updates should not change scroll position"
        );
    }
}

// ─── Auto-scroll on MessageEnd ───────────────────────────────────────────────

mod auto_scroll_message_end {
    use super::*;

    #[test]
    fn test_message_end_resets_scroll_when_at_bottom() {
        let mut state = make_state();
        add_messages(&mut state, 20);
        state.scroll.feed_offset = 0;
        state.scroll.user_scrolled_up = false;

        on_message_end(&mut state, make_agent_message("Final response"));

        assert_eq!(state.scroll.feed_offset, 0, "MessageEnd should keep offset at 0 when at bottom");
    }

    #[test]
    fn test_message_end_does_not_scroll_when_user_scrolled_up() {
        let mut state = make_state();
        add_messages(&mut state, 50);
        state.scroll.feed_offset = 35;
        state.scroll.user_scrolled_up = true;

        on_message_end(&mut state, make_agent_message("Final"));

        assert_eq!(
            state.scroll.feed_offset, 35,
            "MessageEnd should NOT reset when user scrolled up"
        );
    }

    #[test]
    fn test_message_end_clears_is_thinking() {
        let mut state = make_state();
        state.thinking = Some(ThinkingState { start: Some(std::time::Instant::now()), text: String::new(), accrued_duration: None });

        on_message_end(&mut state, make_agent_message("Done"));

        assert!(state.thinking.is_none(), "MessageEnd should clear thinking");
        assert!(state.thinking.as_ref().map_or(true, |t| t.start.is_none()), "MessageEnd should clear thinking.start");
    }

    #[test]
    fn test_message_end_preserves_user_scroll_across_turns() {
        let mut state = make_state();
        add_messages(&mut state, 50);
        state.scroll.feed_offset = 45;
        state.scroll.user_scrolled_up = true;

        // Simulate end of first turn
        on_message_end(&mut state, make_agent_message("Turn 1 done"));

        // User is still scrolled up, next turn should respect that
        state.scroll.feed_offset = 45;
        on_message_start(&mut state, make_agent_message("Turn 2 start"));

        assert_eq!(
            state.scroll.feed_offset, 45,
            "Next turn should respect user's scroll position"
        );
    }
}

// ─── AgentEvent Router Auto-scroll ───────────────────────────────────────────

mod agent_event_auto_scroll {
    use super::*;

    #[test]
    fn test_message_start_event_triggers_auto_scroll() {
        let mut state = make_state();
        add_messages(&mut state, 30);
        state.scroll.feed_offset = 20;
        state.scroll.user_scrolled_up = false;

        let event = AgentEvent::MessageStart {
            message: make_agent_message("Hello"),
            turn: 1,
        };

        handle_agent_event(&mut state, event);

        assert_eq!(state.scroll.feed_offset, 0, "MessageStart event should trigger auto-scroll");
    }

    #[test]
    fn test_message_update_event_triggers_auto_scroll() {
        let mut state = make_state();
        add_messages(&mut state, 30);
        state.scroll.feed_offset = 15;
        state.scroll.user_scrolled_up = false;

        let event = AgentEvent::MessageUpdate {
            message: make_agent_message("Update"),
            turn: 1,
        };

        handle_agent_event(&mut state, event);

        assert_eq!(state.scroll.feed_offset, 0, "MessageUpdate event should trigger auto-scroll");
    }

    #[test]
    fn test_message_end_event_triggers_auto_scroll() {
        let mut state = make_state();
        add_messages(&mut state, 30);
        state.scroll.feed_offset = 25;
        state.scroll.user_scrolled_up = false;

        let event = AgentEvent::MessageEnd {
            message: make_agent_message("Done"),
            turn: 1,
        };

        handle_agent_event(&mut state, event);

        assert_eq!(state.scroll.feed_offset, 0, "MessageEnd event should trigger auto-scroll");
    }

    #[test]
    fn test_non_message_events_do_not_affect_scroll() {
        let mut state = make_state();
        add_messages(&mut state, 30);
        state.scroll.feed_offset = 20;
        state.scroll.user_scrolled_up = false;

        // Tool events should not affect scroll
        let event = AgentEvent::ToolExecutionStart {
            tool_call_id: "tool_1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "ls".to_string(),
            turn: 1,
        };

        handle_agent_event(&mut state, event);

        // Scroll should remain unchanged
        assert_eq!(state.scroll.feed_offset, 20);
        assert!(!state.scroll.user_scrolled_up);
    }
}

// ─── Auto-scroll Disabled When User Scrolled ─────────────────────────────────

mod auto_scroll_disabled_when_user_scrolled {
    use super::*;

    #[test]
    fn test_all_message_events_respect_user_scrolled_up() {
        let mut state = make_state();
        add_messages(&mut state, 50);

        // Test each message event type
        for offset in [10, 20, 30, 40] {
            state.scroll.feed_offset = offset;
            state.scroll.user_scrolled_up = true;

            let start_event = AgentEvent::MessageStart {
                message: make_agent_message("Start"),
                turn: 1,
            };
            handle_agent_event(&mut state, start_event);
            assert_eq!(
                state.scroll.feed_offset, offset,
                "MessageStart should not scroll when user_scrolled_up"
            );

            state.scroll.feed_offset = offset;
            let update_event = AgentEvent::MessageUpdate {
                message: make_agent_message("Update"),
                turn: 1,
            };
            handle_agent_event(&mut state, update_event);
            assert_eq!(
                state.scroll.feed_offset, offset,
                "MessageUpdate should not scroll when user_scrolled_up"
            );

            state.scroll.feed_offset = offset;
            let end_event = AgentEvent::MessageEnd {
                message: make_agent_message("End"),
                turn: 1,
            };
            handle_agent_event(&mut state, end_event);
            assert_eq!(
                state.scroll.feed_offset, offset,
                "MessageEnd should not scroll when user_scrolled_up"
            );
        }
    }

    #[test]
    fn test_user_scrolled_up_blocks_auto_scroll_but_not_manual() {
        let mut state = make_state();
        add_messages(&mut state, 50);
        state.scroll.feed_offset = 40;
        state.scroll.user_scrolled_up = true;

        // Auto-scroll via MessageStart should be blocked
        let event = AgentEvent::MessageStart {
            message: make_agent_message("Test"),
            turn: 1,
        };
        handle_agent_event(&mut state, event.clone());
        assert_eq!(state.scroll.feed_offset, 40);

        // But manual scroll down should still work
        state.scroll.user_scrolled_up = false;
        state.scroll.feed_offset = 40;
        handle_agent_event(&mut state, event);
        assert_eq!(state.scroll.feed_offset, 0);
    }
}