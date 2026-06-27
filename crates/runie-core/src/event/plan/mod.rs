//! Plan-related events for plan-first execution mode.

use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::actors::plan::{PlanState, PlanStep, PlanStepStatus};

/// Plan-related events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoStaticStr)]
#[serde(tag = "type", content = "data")]
pub enum PlanEvent {
    /// Plan was created.
    PlanCreated {
        id: String,
        title: String,
        steps: Vec<PlanStep>,
    },
    /// Plan was modified (step added/updated, status changed).
    PlanChanged {
        plan: PlanState,
    },
    /// Plan was submitted for approval.
    PlanSubmitted,
    /// Plan was approved — write tools are now unblocked.
    PlanApproved {
        plan: PlanState,
    },
    /// Plan was rejected — write tools remain blocked.
    PlanRejected {
        plan: PlanState,
    },
    /// Plan completed successfully (all steps done).
    PlanCompleted {
        plan: PlanState,
    },
    /// Plan was cleared/reset.
    PlanCleared,
    /// A step was added to the plan.
    PlanStepAdded {
        id: usize,
        description: String,
        depends_on: Vec<usize>,
    },
    /// A step's status was updated.
    PlanStepUpdated {
        step_id: usize,
        status: PlanStepStatus,
    },
}
