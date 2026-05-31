//! Lifecycle tests.
//!
//! Tests:
//! - AgentEnd cleanup (status, mode, placeholder removal)
//! - TurnEnd adds separator with metrics
//! - Multiple agent end/start cycles
//! - AgentEnd while in Permission mode clears permission

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::state::TuiMode;
use crate::tui::update::agent::handle_agent_event;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, TokenUsage};

/// Helper: Create an AgentMessage with given role and content text.
fn agent_message(role: &str, text: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![ContentPart::Text {
            text: text.to_string(),
        }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

/// Helper: Create AppState ready for lifecycle testing.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("test-model".to_string());
    state
}

// ─── AgentEnd cleanup tests ──────────────────────────────────────────────────

#[test]
fn test_agent_end_clears_agent_running() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.is_thinking = true;
    state.thinking_start = Some(std::time::Instant::now());
    state.status_header = Some("Thinking".to_string());
    state.status_start_time = Some(std::time::Instant::now());

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert!(!state.agent_running, "agent_running should be false");
}

#[test]
fn test_agent_end_clears_thinking_state() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.is_thinking = true;
    state.thinking_start = Some(std::time::Instant::now());
    state.thinking_duration = Some(std::time::Duration::from_millis(100));

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert!(!state.is_thinking, "is_thinking should be false");
    assert!(state.thinking_start.is_none(), "thinking_start should be none");
}

#[test]
fn test_agent_end_clears_status() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.status_header = Some("Thinking".to_string());
    state.status_details = Some("Running tool".to_string());
    state.status_start_time = Some(std::time::Instant::now());

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert!(state.status_header.is_none(), "status_header should be none");
    assert!(state.status_details.is_none(), "status_details should be none");
    assert!(state.status_start_time.is_none(), "status_start_time should be none");
}

#[test]
fn test_agent_end_clears_agent_start_time() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.agent_start_time = Some(std::time::Instant::now());

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert!(state.agent_start_time.is_none(), "agent_start_time should be none");
}

#[test]
fn test_agent_end_resets_mode_to_chat() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.mode = TuiMode::Permission;

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert_eq!(state.mode, TuiMode::Chat, "mode should return to Chat");
}

#[test]
fn test_agent_end_removes_empty_placeholder() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.messages.push(MessageItem::Assistant {
        text: String::new(),
        model: Some("test-model".to_string()),
        timestamp: None,
    });

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert!(
        !state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text.is_empty()
        )),
        "empty placeholder should be removed"
    );
}

#[test]
fn test_agent_end_preserves_non_empty_placeholder() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.messages.push(MessageItem::Assistant {
        text: "Hello".to_string(),
        model: Some("test-model".to_string()),
        timestamp: None,
    });

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert!(
        state.messages.iter().any(|m| matches!(
            m,
            MessageItem::Assistant { text, .. } if text == "Hello"
        )),
        "non-empty assistant should be preserved"
    );
}

#[test]
fn test_agent_end_clears_permission_modal() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("call-1".to_string());

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert!(state.permission_modal.tool.is_none(), "permission tool should be cleared");
    assert!(state.permission_modal.tool_call_id.is_none(), "permission tool_call_id should be cleared");
}

#[test]
fn test_agent_end_clears_permission_queue() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "call-2".to_string(),
        tool_name: "read_file".to_string(),
        tool_args: "{}".to_string(),
    });
    state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "call-3".to_string(),
        tool_name: "write_file".to_string(),
        tool_args: "{}".to_string(),
    });

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert!(
        state.permission_modal.pending_queue.is_empty(),
        "pending permission queue should be cleared"
    );
}

#[test]
fn test_agent_end_does_not_clear_current_model() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.current_model = Some("gpt-4o".to_string());

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert_eq!(
        state.current_model.as_deref(),
        Some("gpt-4o"),
        "current_model should persist"
    );
}

// ─── TurnEnd tests ────────────────────────────────────────────────────────────

#[test]
fn test_turn_end_adds_separator() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.agent_start_time = Some(std::time::Instant::now());
    state.session_token_usage.total_tokens = 1000;

    // Add a tool call to verify tool_calls count
    state.messages.push(MessageItem::ToolCall {
        name: "call-1".to_string(),
        args: "{}".to_string(),
        result: Some("result".to_string()),
        is_error: false,
    });

    handle_agent_event(
        &mut state,
        AgentEvent::TurnEnd {
            turn: 1,
            message_count: 5,
            tool_results_count: 1,
            token_usage: TokenUsage::default(),
        },
    );

    assert!(
        state.messages.iter().any(|m| matches!(m, MessageItem::Separator { .. })),
        "should have separator after turn end"
    );
}

#[test]
fn test_turn_end_separator_contains_metrics() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.agent_start_time = Some(std::time::Instant::now());
    state.session_token_usage.total_tokens = 500;

    state.messages.push(MessageItem::ToolCall {
        name: "call-1".to_string(),
        args: "{}".to_string(),
        result: Some("result".to_string()),
        is_error: false,
    });

    handle_agent_event(
        &mut state,
        AgentEvent::TurnEnd {
            turn: 1,
            message_count: 3,
            tool_results_count: 1,
            token_usage: TokenUsage {
                input: 100,
                output: 200,
                total_tokens: 360,
                cache_read: 0,
                cache_write: 0,
            },
        },
    );

    let separator = state.messages.iter().find_map(|m| match m {
        MessageItem::Separator {
            elapsed_secs,
            tool_calls,
            tokens_used,
        } => Some((*elapsed_secs, *tool_calls, *tokens_used)),
        _ => None,
    });

    assert!(separator.is_some(), "should have separator with metrics");
    let (_elapsed, tool_calls, tokens) = separator.unwrap();
    assert_eq!(tool_calls, 1, "tool_calls should be 1");
    assert_eq!(tokens, Some(500), "tokens_used should match session_token_usage");
}

#[test]
fn test_turn_end_without_agent_start_time_no_separator() {
    let mut state = make_test_state();
    state.agent_running = true;
    // No agent_start_time set

    handle_agent_event(
        &mut state,
        AgentEvent::TurnEnd {
            turn: 1,
            message_count: 5,
            tool_results_count: 0,
            token_usage: TokenUsage::default(),
        },
    );

    assert!(
        !state.messages.iter().any(|m| matches!(m, MessageItem::Separator { .. })),
        "should not have separator without agent_start_time"
    );
}

// ─── Multiple lifecycle cycles ────────────────────────────────────────────────

#[test]
fn test_multiple_agent_start_end_cycles() {
    let mut state = make_test_state();

    for i in 1..=3 {
        // Start agent
        handle_agent_event(
            &mut state,
            AgentEvent::MessageStart {
                message: agent_message("assistant", ""),
                turn: i,
            },
        );
        assert!(state.agent_running, "agent should be running in turn {}", i);

        // End agent
        handle_agent_event(
            &mut state,
            AgentEvent::AgentEnd {
                messages: vec![],
                total_turns: i,
                final_token_usage: TokenUsage::default(),
            },
        );
        assert!(!state.agent_running, "agent should not be running after turn {}", i);
    }

    // Empty assistants are replaced with system messages per design
    assert_eq!(
        state.messages.iter().filter(|m| matches!(m, MessageItem::System { .. })).count(),
        3,
        "should have 3 system messages (empty assistants replaced)"
    );
}

#[test]
fn test_rapid_agent_end_start() {
    let mut state = make_test_state();

    for _ in 0..10 {
        state.agent_running = true;
        handle_agent_event(
            &mut state,
            AgentEvent::AgentEnd {
                messages: vec![],
                total_turns: 1,
                final_token_usage: TokenUsage::default(),
            },
        );
    }

    assert!(!state.agent_running, "agent should not be running");
}

// ─── Lifecycle with messages ─────────────────────────────────────────────────

#[test]
fn test_agent_end_with_final_messages() {
    let mut state = make_test_state();
    state.agent_running = true;

    let final_messages = vec![
        agent_message("assistant", "First response"),
        agent_message("assistant", "Second response"),
    ];

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: final_messages.clone(),
            total_turns: 2,
            final_token_usage: TokenUsage::default(),
        },
    );

    assert!(!state.agent_running, "agent should not be running");
    // Note: AgentEnd doesn't automatically add the messages from final_token_usage
    // to the UI - that's handled by the message flow events
}

#[test]
fn test_agent_end_with_token_usage() {
    let mut state = make_test_state();
    state.agent_running = true;

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage {
                input: 500,
                output: 1000,
                total_tokens: 1750,
                cache_read: 0,
                cache_write: 0,
            },
        },
    );

    // AgentEnd itself doesn't update token usage - that's done via TokenUsage events
    // But it should complete without error
    assert!(!state.agent_running);
}

// ─── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn test_agent_end_without_any_prior_state() {
    let mut state = AppState::default();
    // No agent_running, no messages, etc.

    handle_agent_event(
        &mut state,
        AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 0,
            final_token_usage: TokenUsage::default(),
        },
    );

    // Should handle gracefully
    assert!(!state.agent_running);
}

#[test]
fn test_turn_end_during_active_thinking() {
    let mut state = make_test_state();
    state.agent_running = true;
    state.is_thinking = true;
    state.thinking_start = Some(std::time::Instant::now());
    state.agent_start_time = Some(std::time::Instant::now());

    handle_agent_event(
        &mut state,
        AgentEvent::TurnEnd {
            turn: 1,
            message_count: 2,
            tool_results_count: 0,
            token_usage: TokenUsage::default(),
        },
    );

    // TurnEnd should not affect thinking state
    assert!(state.is_thinking, "is_thinking should remain unchanged");
}
