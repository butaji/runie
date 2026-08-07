//! Read-only projection of agent state.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{AgentMessage, AgentTool, Model, ThinkingLevel};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkflowSnapshot {
    pub name: String,
    pub objective: String,
    pub phase: Option<String>,
    pub state: Option<String>,
    pub active_agents: u32,
    pub status: String,
    pub elapsed_ms: Option<u64>,
}

/// Immutable projection of one Pi background-work lifecycle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackgroundWorkSnapshot {
    pub description: String,
    pub activity: Option<String>,
    pub background: bool,
    pub status: String,
    pub elapsed_ms: Option<u64>,
    pub error: Option<String>,
}

/// Immutable view of agent state. Projected by `AgentStateActor`; consumed by
/// hooks, drivers, and tests.
#[derive(Clone, Default)]
pub struct AgentStateSnapshot {
    pub system_prompt: String,
    pub model: Model,
    pub thinking_level: ThinkingLevel,
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<Arc<dyn AgentTool>>,
    pub is_streaming: bool,
    pub streaming_message: Option<AgentMessage>,
    pub pending_tool_calls: Vec<String>,
    pub error_message: Option<String>,
    pub workflows: HashMap<String, WorkflowSnapshot>,
    pub background_work: HashMap<String, BackgroundWorkSnapshot>,
}

impl AgentStateSnapshot {
    pub fn pending_count(&self) -> usize {
        self.pending_tool_calls.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_snapshot_is_idle() {
        let s = AgentStateSnapshot::default();
        assert!(!s.is_streaming);
        assert!(s.error_message.is_none());
        assert_eq!(s.pending_count(), 0);
        assert!(s.workflows.is_empty());
        assert!(s.background_work.is_empty());
    }
}
