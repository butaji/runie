//! Pure turn-planning decision fn.

use crate::state::AgentStateSnapshot;
use crate::types::{AgentMessage, ToolCall};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnPlan {
    /// Continue to the next turn.
    Continue,
    /// Stop the loop (graceful).
    Stop { reason: &'static str },
    /// Tool calls from the assistant need dispatching.
    ToolBatch { calls: Vec<ToolCall> },
}

/// Decide what the loop should do next based on the latest snapshot and
/// queue state. Pure function — no IO.
pub fn decide_next_turn(
    snapshot: &AgentStateSnapshot,
    pending_calls: Vec<ToolCall>,
    steering_empty: bool,
    follow_up_empty: bool,
) -> TurnPlan {
    if !pending_calls.is_empty() {
        return TurnPlan::ToolBatch {
            calls: pending_calls,
        };
    }
    if !steering_empty || !follow_up_empty {
        return TurnPlan::Continue;
    }
    if snapshot.error_message.is_some() {
        return TurnPlan::Stop { reason: "error" };
    }
    TurnPlan::Stop {
        reason: "no-more-work",
    }
}

/// Convenience: count of messages added in a run for diagnostic purposes.
pub fn new_messages_count(initial: &[AgentMessage], current: &[AgentMessage]) -> usize {
    current.len().saturating_sub(initial.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_snapshot_with_empty_queues_stops() {
        let s = AgentStateSnapshot::default();
        match decide_next_turn(&s, vec![], true, true) {
            TurnPlan::Stop { .. } => {}
            other => panic!("expected Stop, got {other:?}"),
        }
    }

    #[test]
    fn pending_calls_force_tool_batch() {
        let s = AgentStateSnapshot::default();
        let calls = vec![ToolCall {
            id: "x".into(),
            name: "echo".into(),
            arguments: serde_json::json!({}),
            thought_signature: None,
        }];
        assert!(matches!(
            decide_next_turn(&s, calls, true, true),
            TurnPlan::ToolBatch { .. }
        ));
    }

    #[test]
    fn steering_message_continues() {
        let s = AgentStateSnapshot::default();
        assert!(matches!(
            decide_next_turn(&s, vec![], false, true),
            TurnPlan::Continue
        ));
    }
}
