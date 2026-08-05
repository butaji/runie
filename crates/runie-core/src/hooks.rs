//! Default hook implementations.

use crate::types::{BeforeToolCallContext, BeforeToolCallResult};

/// Default `before_tool_call`: always returns `None` (allow).
pub fn default_before_tool_call(_ctx: BeforeToolCallContext) -> BeforeToolCallResult {
    BeforeToolCallResult::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_before_returns_allow() {
        let r = default_before_tool_call(BeforeToolCallContext {
            assistant_message: crate::types::AssistantMessage {
                content: vec![],
                stop_reason: None,
                model: "test".into(),
                timestamp: 0,
                ..Default::default()
            },
            tool_call: crate::types::ToolCall {
                id: "x".into(),
                name: "echo".into(),
                arguments: serde_json::json!({}),
            },
            args: serde_json::json!({}),
        });
        assert!(!r.block);
    }
}
