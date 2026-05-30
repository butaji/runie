//! Comprehensive test suite - Section 5: Cancellation/Timeout Tests (pi pattern).

use crate::components::MessageItem;
use crate::tui::state::Cmd;
use crate::tui::update::system::check_agent_timeout;

use super::harness::AgentTestHarness;
use super::state_tests::make_message;

#[test]
fn test_agent_timeout() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(runie_agent::AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    harness.assert_agent_running();

    // Simulate timeout by setting agent_start_time to 601 seconds ago
    harness.state.agent_start_time =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(601));

    let cmds = check_agent_timeout(&mut harness.state);

    assert!(!harness.state.agent_running);
    assert!(cmds.is_some());
    assert!(cmds.unwrap().contains(&Cmd::Interrupt));
}

#[test]
fn test_agent_timeout_clears_state() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(runie_agent::AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    harness = harness.handle_event(runie_agent::AgentEvent::MessageEnd {
        message: make_message("assistant", "Done"),
        turn: 1,
    });

    harness.state.is_thinking = true;
    harness.state.thinking_start =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(600));
    harness.state.status_header = Some("Thinking".to_string());

    harness.state.agent_start_time =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(601));

    check_agent_timeout(&mut harness.state);

    assert!(!harness.state.agent_running);
    assert!(!harness.state.is_thinking);
    assert!(harness.state.thinking_start.is_none());
    assert!(harness.state.status_header.is_none());
}

#[test]
fn test_abort_clears_queues() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(runie_agent::AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    harness.state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
    });

    harness = harness.handle_event(runie_agent::AgentEvent::MessageEnd {
        message: make_message("assistant", "Done"),
        turn: 1,
    });

    harness = harness.handle_event(runie_agent::AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: runie_agent::TokenUsage::default(),
    });

    assert!(!harness.state.agent_running);
    assert!(!harness.state.is_thinking);
    assert!(harness.state.thinking_start.is_none());
    assert!(harness.state.permission_modal.pending_queue.is_empty());
}

#[test]
fn test_permission_queue_cleared_on_timeout() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness.state.permission_modal.pending_queue.push(crate::tui::state::PendingPermission {
        tool_call_id: "t1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "ls".to_string(),
    });

    harness.state.agent_start_time =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(601));
    harness.state.agent_running = true;

    check_agent_timeout(&mut harness.state);

    assert!(harness.state.permission_modal.pending_queue.is_empty());
}

#[test]
fn test_no_timeout_when_not_running() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness.state.agent_start_time =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(601));

    let cmds = check_agent_timeout(&mut harness.state);

    assert!(cmds.is_none());
}

#[test]
fn test_no_timeout_within_limit() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(runie_agent::AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    // Set start time to only 100 seconds ago (under 600s limit)
    harness.state.agent_start_time =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(100));

    let cmds = check_agent_timeout(&mut harness.state);

    assert!(cmds.is_none());
    assert!(harness.state.agent_running);
}

#[test]
fn test_timeout_adds_system_message() {
    let mut harness = AgentTestHarness::new();
    harness = harness.user_says("Hello");

    harness = harness.handle_event(runie_agent::AgentEvent::MessageStart {
        message: make_message("assistant", ""),
        turn: 1,
    });

    harness.state.agent_start_time =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(601));

    check_agent_timeout(&mut harness.state);

    let has_timeout_msg = harness.state.messages.iter().any(|m| {
        matches!(m, MessageItem::System { text } if text.contains("timed out"))
    });

    assert!(has_timeout_msg);
}
