//! Default hook implementations + turn-level hook types.

use std::sync::Arc;

use crate::types::{
    AgentContext, AgentMessage, AssistantMessage, BeforeToolCallContext, BeforeToolCallResult,
    Model, ThinkingLevel, ToolResultMessage,
};

/// Default `before_tool_call`: always returns `None` (allow).
pub fn default_before_tool_call(_ctx: BeforeToolCallContext) -> BeforeToolCallResult {
    BeforeToolCallResult::default()
}

/// Input to `shouldStopAfterTurn` / `prepareNextTurn` (pi types.ts:121,142).
#[derive(Clone)]
pub struct ShouldStopAfterTurnContext {
    pub message: AssistantMessage,
    pub tool_results: Vec<ToolResultMessage>,
    pub context: AgentContext,
    pub new_messages: Vec<AgentMessage>,
}

/// `prepareNextTurn` input is the same shape (pi `PrepareNextTurnContext`).
pub type PrepareNextTurnContext = ShouldStopAfterTurnContext;

/// Returned by `prepareNextTurn` (pi `AgentLoopTurnUpdate`, types.ts:133).
#[derive(Clone, Default)]
pub struct TurnUpdate {
    pub context: Option<AgentContext>,
    pub model: Option<Model>,
    pub thinking_level: Option<ThinkingLevel>,
}

/// Turn-level hooks consulted after each `turn_end` (pi `agent-loop.ts:232,247`).
#[derive(Default, Clone)]
pub struct TurnHooks {
    /// Called after `turn_end`; returning `Some(update)` replaces the
    /// context/model/thinking level for the following turn.
    pub prepare_next_turn:
        Option<Arc<dyn Fn(PrepareNextTurnContext) -> Option<TurnUpdate> + Send + Sync>>,
    /// Called after `turn_end`; returning `true` ends the agent immediately
    /// (before the steering/follow-up poll).
    pub should_stop_after_turn:
        Option<Arc<dyn Fn(ShouldStopAfterTurnContext) -> bool + Send + Sync>>,
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
