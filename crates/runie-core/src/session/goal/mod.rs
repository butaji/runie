//! Goal Mode Orchestration Module.
//!
//! Provides multi-phase goal execution with planning, execution, and verification:
//! - [`GoalPhase`]: Idle | Planning | Executing | Paused | Completed | Failed
//! - [`GoalStatus`]: Active | UserPaused | BackOffPaused | NoProgressPaused | InfraPaused | Blocked | BudgetLimited | Complete
//! - [`GoalTracker`]: State machine with transitions and persistence
//! - [`GoalPlanner`]: Decomposes goal into structured task list
//!
//! Re-exports from the parent `crate::goal` for convenience.

pub mod planner;

pub use crate::goal::{
    Checkpoint, GoalPhase, GoalRole, GoalState, GoalStatus, GoalTracker,
};
