//! PlanActor message types.

use serde::{Deserialize, Serialize};

use super::state::PlanStepStatus;

/// Messages accepted by PlanActor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanMsg {
    /// Create a new plan.
    CreatePlan {
        id: String,
        title: String,
    },
    /// Add a step to the current plan.
    AddStep {
        description: String,
        depends_on: Vec<usize>,
    },
    /// Submit the plan for approval.
    SubmitPlan,
    /// Approve the plan.
    ApprovePlan,
    /// Reject the plan.
    RejectPlan,
    /// Update a step's status (from TurnActor or tool execution).
    UpdateStep {
        step_id: usize,
        status: PlanStepStatus,
    },
    /// Mark a step as started (executing).
    StartStep {
        step_id: usize,
    },
    /// Mark a step as completed.
    CompleteStep {
        step_id: usize,
    },
    /// Mark a step as failed.
    FailStep {
        step_id: usize,
        error: String,
    },
    /// Clear/reset the plan.
    ClearPlan,
    /// Check plan status and emit appropriate event.
    CheckStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_serialize_and_deserialize() {
        let msg = PlanMsg::CreatePlan {
            id: "p1".into(),
            title: "Test plan".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let round_trip: PlanMsg = serde_json::from_str(&json).unwrap();
        assert!(matches!(round_trip, PlanMsg::CreatePlan { id, title }
            if id == "p1" && title == "Test plan"));
    }

    #[test]
    fn step_status_serializes() {
        use super::super::state::PlanStepStatus;
        let status = PlanStepStatus::Failed {
            error: "oops".into(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Failed"));
        assert!(json.contains("oops"));
    }
}
