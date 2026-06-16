//! Sidebar events for Team mode subagent panel.

use serde::{Deserialize, Serialize};
use crate::state::{AgentEntry, AgentStatus};

/// Events for the subagent sidebar in Team mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SidebarEvent {
    /// Show the sidebar (Team mode entered or plan started).
    Show,
    /// Hide the sidebar (Team mode exited).
    Hide,
    /// Switch focus to the Orchestrator feed (Ctrl+0).
    FocusOrchestrator,
    /// Switch focus to a subagent by 1-based index (Ctrl+1..9).
    FocusSubagent(usize),
    /// Update an agent's status in the sidebar.
    UpdateStatus { id: String, status: AgentStatus },
    /// Set the full list of subagents (replaces indices 1+).
    SetSubagents(Vec<AgentEntry>),
    /// Set the Orchestrator's status.
    SetOrchestratorStatus(AgentStatus),
}
