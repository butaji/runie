//! Extension traits for test assertions on AgentTestHarness.

use super::test_harness::AgentTestHarness;
use crate::components::MessageItem;

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
