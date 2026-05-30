use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use crate::tui::state::Cmd;
use crate::tui::update::system::check_agent_timeout;
use runie_agent::AgentEvent;
use std::time::{Duration, Instant};

#[test]
fn test_error_recovery() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Hello");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 });
    harness.assert_agent_running();

    let harness = harness.handle_agent_event(AgentEvent::Error {
        message: "Network error".to_string(),
        error_type: "network".to_string(),
        recoverable: true,
        context: "".to_string(),
    });

    assert!(!harness.state.agent_running, "agent_running should be false after error");
    assert!(harness.state.status_header.is_none(), "status_header should be None after error");
    assert!(!harness.state.is_thinking, "is_thinking should be false after error");
    assert!(harness.state.messages.iter().any(|m| matches!(m, MessageItem::Error { .. })), "should have an Error message item");
}

#[test]
fn test_non_recoverable_error() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Hello");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 });
    let harness = harness.handle_agent_event(AgentEvent::Error {
        message: "Internal panic".to_string(),
        error_type: "panic".to_string(),
        recoverable: false,
        context: "".to_string(),
    });

    let error_item = harness.state.messages.iter().rev().find_map(|m| match m {
        MessageItem::Error { recoverable, .. } => Some(recoverable),
        _ => None,
    });
    assert_eq!(error_item, Some(&false), "error should be marked as non-recoverable");
}

#[test]
fn test_timeout_clears_state() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Hello");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 });
    harness.assert_agent_running();

    let mut harness = harness;
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(601));

    let cmds = check_agent_timeout(&mut harness.state);

    assert!(!harness.state.agent_running, "agent_running should be false after timeout");
    assert!(harness.state.agent_start_time.is_none(), "agent_start_time should be cleared");
    assert!(!harness.state.is_thinking, "is_thinking should be false after timeout");
    assert!(cmds.is_some() && cmds.as_ref().unwrap().contains(&Cmd::Interrupt), "should return Cmd::Interrupt");
}

#[test]
fn test_timeout_adds_system_message() {
    let harness = AgentTestHarness::new();
    let harness = harness.submit_user_message("Hello");

    let harness = harness.handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn: 1 });

    let mut harness = harness;
    harness.state.agent_start_time = Some(Instant::now() - Duration::from_secs(601));
    check_agent_timeout(&mut harness.state);

    let has_timeout_msg = harness.state.messages.iter().any(|m| match m {
        MessageItem::System { text } => text.contains("timed out"),
        _ => false,
    });
    assert!(has_timeout_msg, "should have a system message about timeout");
}
