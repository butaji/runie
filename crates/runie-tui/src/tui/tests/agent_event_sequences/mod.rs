//! Agent event sequence tests module.
//!
//! This module contains comprehensive tests for agent event sequences.
//! Split into submodules for better organization.

pub mod helpers;
pub mod basic_flow_tests;
pub mod tool_execution_tests;
pub mod error_recovery_tests;
pub mod lifecycle_tests;

pub use helpers::{agent_message, tool_result, turn_end_event};

// Extension traits for test assertions
impl AgentTestHarness {
    /// Assert harness has no agent running
    pub fn assert_agent_not_running(&self) {
        assert!(
            !self.state.agent_running,
            "agent should NOT be running"
        );
    }

    /// Assert harness has a user message containing the given text
    pub fn assert_has_user_message(&self, text: &str) {
        let has_message = self.state.messages.iter().any(|m| match m {
            MessageItem::User { text: t, .. } => t.contains(text),
            _ => false,
        });
        assert!(
            has_message,
            "should have user message containing: {}",
            text
        );
    }
}

use crate::components::MessageItem;
use crate::tui::tests::test_harness::AgentTestHarness;
