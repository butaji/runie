//! Orchestrator domain types for Team mode execution.
//!
//! Defines the data structures that represent an Orchestrator plan:
//! `OrchestratorPlan`, `SubagentTask`, `TaskStatus`, and `PlanResult`.
//! These types are serializable and have no TUI dependencies.
//!
//! The orchestrator is the top-level planner in Team mode. It decomposes a
//! user goal into a list of `SubagentTask` items, each assigned to a
//! `ModelTrait`. The harness resolves traits to concrete (provider, model)
//! pairs using the connected model catalog.

use serde::{Deserialize, Serialize};

mod context;
mod lifecycle;
mod plan;
mod task;

pub use context::{DialogueEntry, OrchestratorContext};
pub use lifecycle::AgentLifecycleStatus;
pub use plan::{OrchestratorPlan, PlanResult, TaskFailure};
pub use task::{ModelTrait, SubagentTask};

/// Backward-compatible alias used by plan/task code.
pub type TaskStatus = AgentLifecycleStatus;

// ─────────────────────────────────────────────────────────────────────────────
// Execution mode
// ─────────────────────────────────────────────────────────────────────────────

/// Execution mode for the current session.
///
/// - **Solo**: One agent does planning and execution. This is the default.
/// - **Team**: An Orchestrator plans a workflow, spawns subagents, and
///   synthesizes results. User-facing in the status bar and toggled via
///   `/team` and `/solo` commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// One agent, one turn — the default.
    #[default]
    Solo,
    /// Orchestrator + subagents workflow.
    Team,
}

impl ExecutionMode {
    /// Short label for status bar display.
    pub fn status_label(&self) -> &'static str {
        match self {
            ExecutionMode::Solo => "solo",
            ExecutionMode::Team => "team",
        }
    }

    /// Whether this mode uses the Orchestrator.
    pub fn uses_orchestrator(&self) -> bool {
        matches!(self, ExecutionMode::Team)
    }
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionMode::Solo => write!(f, "solo"),
            ExecutionMode::Team => write!(f, "team"),
        }
    }
}
