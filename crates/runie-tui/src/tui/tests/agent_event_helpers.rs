//! Helper functions for agent event sequence tests.

use crate::tui::tests::test_harness::AgentTestHarness;
use crate::components::MessageItem;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult, TokenUsage};

/// Create a test AgentMessage with the given role and text content
pub fn agent_message(role: &str, text: &str) -> AgentMessage {
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

/// Create a minimal ToolResult for testing
pub fn tool_result(content: &str, is_error: bool) -> ToolResult {
    ToolResult {
        tool_call_id: "call-1".to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({}),
        content: vec![ContentPart::Text {
            text: content.to_string(),
        }],
        is_error,
    }
}

/// Create a TurnEnd event with reasonable defaults
pub fn turn_end_event(turn: usize) -> AgentEvent {
    AgentEvent::TurnEnd {
        turn,
        message_count: 2,
        tool_results_count: 0,
        token_usage: TokenUsage {
            input: 100,
            output: 50,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 150,
        },
    }
}
