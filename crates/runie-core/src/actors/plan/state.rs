//! Plan state types.

use serde::{Deserialize, Serialize};

/// A node in the plan graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanStep {
    /// Unique step identifier.
    pub id: usize,
    /// Human-readable description of the step.
    pub description: String,
    /// Current status of the step.
    pub status: PlanStepStatus,
    /// Dependencies on other steps (must complete before this starts).
    #[serde(default)]
    pub depends_on: Vec<usize>,
}

/// Status of a plan step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStepStatus {
    /// Step is pending, waiting for dependencies.
    Pending,
    /// Step is approved and can execute.
    Approved,
    /// Step is currently executing.
    Executing,
    /// Step completed successfully.
    Completed,
    /// Step failed.
    Failed { error: String },
    /// Step was skipped.
    Skipped,
}

impl Default for PlanStepStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Overall plan status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    /// Plan is being created/edited.
    Drafting,
    /// Plan is submitted for approval.
    PendingApproval,
    /// Plan is approved and steps can execute.
    Approved,
    /// Plan is rejected.
    Rejected,
    /// Plan completed successfully.
    Completed,
    /// Plan failed.
    Failed { error: String },
}

impl Default for PlanStatus {
    fn default() -> Self {
        Self::Drafting
    }
}

/// The plan graph owned by PlanActor.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlanState {
    /// Plan identifier.
    pub id: String,
    /// Plan title/summary.
    pub title: String,
    /// All steps in the plan.
    pub steps: Vec<PlanStep>,
    /// Current plan status.
    pub status: PlanStatus,
    /// Whether write tools are currently blocked.
    pub write_blocked: bool,
}

impl PlanState {
    /// Create a new plan with the given title.
    pub fn new(id: String, title: String) -> Self {
        Self {
            id,
            title,
            steps: Vec::new(),
            status: PlanStatus::Drafting,
            write_blocked: true,
        }
    }

    /// Add a step to the plan.
    pub fn add_step(&mut self, description: String, depends_on: Vec<usize>) -> usize {
        let id = self.steps.len();
        self.steps.push(PlanStep {
            id,
            description,
            status: PlanStepStatus::Pending,
            depends_on,
        });
        self.status = PlanStatus::Drafting;
        self.write_blocked = true;
        id
    }

    /// Update a step's status.
    pub fn update_step_status(&mut self, step_id: usize, status: PlanStepStatus) -> bool {
        match self.steps.get_mut(step_id) {
            Some(step) => {
                step.status = status;
                true
            }
            None => false,
        }
    }

    /// Check if all steps are complete.
    pub fn all_steps_complete(&self) -> bool {
        self.steps.iter().all(|s| {
            matches!(
                s.status,
                PlanStepStatus::Completed | PlanStepStatus::Skipped | PlanStepStatus::Pending
            ) && !matches!(s.status, PlanStepStatus::Failed { .. })
        }) && !self.steps.is_empty()
    }

    /// Check if a step can execute (dependencies met and approved).
    pub fn can_execute(&self, step_id: usize) -> bool {
        let Some(step) = self.steps.get(step_id) else {
            return false;
        };
        if self.status != PlanStatus::Approved {
            return false;
        }
        if !matches!(step.status, PlanStepStatus::Approved | PlanStepStatus::Pending) {
            return false;
        }
        step.depends_on.iter().all(|dep_id| {
            self.steps
                .get(*dep_id)
                .map(|s| matches!(s.status, PlanStepStatus::Completed | PlanStepStatus::Skipped))
                .unwrap_or(true)
        })
    }

    /// Get the next executable step.
    pub fn next_executable_step(&self) -> Option<usize> {
        self.steps
            .iter()
            .position(|s| self.can_execute(s.id))
    }

    /// Approve the plan for execution.
    pub fn approve(&mut self) {
        self.status = PlanStatus::Approved;
        self.write_blocked = false;
        // Mark pending steps without unmet dependencies as approved
        let ready_ids: Vec<usize> = self
            .steps
            .iter()
            .filter(|s| matches!(s.status, PlanStepStatus::Pending) && self.can_execute(s.id))
            .map(|s| s.id)
            .collect();

        for step_id in ready_ids {
            if let Some(step) = self.steps.get_mut(step_id) {
                step.status = PlanStepStatus::Approved;
            }
        }
    }

    /// Reject the plan.
    pub fn reject(&mut self) {
        self.status = PlanStatus::Rejected;
        self.write_blocked = true;
    }

    /// Finalize the plan as completed.
    pub fn complete(&mut self) {
        self.status = PlanStatus::Completed;
        self.write_blocked = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_plan_is_blocked() {
        let plan = PlanState::new("p1".into(), "Test plan".into());
        assert!(plan.write_blocked);
        assert_eq!(plan.status, PlanStatus::Drafting);
    }

    #[test]
    fn add_step_increments_id() {
        let mut plan = PlanState::new("p1".into(), "Test".into());
        assert_eq!(plan.add_step("Step 1".into(), vec![]), 0);
        assert_eq!(plan.add_step("Step 2".into(), vec![]), 1);
        assert_eq!(plan.add_step("Step 3".into(), vec![]), 2);
        assert_eq!(plan.steps.len(), 3);
    }

    #[test]
    fn approval_unblocks_writes() {
        let mut plan = PlanState::new("p1".into(), "Test".into());
        plan.add_step("Step 1".into(), vec![]);
        assert!(plan.write_blocked);
        plan.approve();
        assert!(!plan.write_blocked);
        assert_eq!(plan.status, PlanStatus::Approved);
    }

    #[test]
    fn rejection_keeps_blocked() {
        let mut plan = PlanState::new("p1".into(), "Test".into());
        plan.add_step("Step 1".into(), vec![]);
        plan.approve();
        assert!(!plan.write_blocked);
        plan.reject();
        assert!(plan.write_blocked);
        assert_eq!(plan.status, PlanStatus::Rejected);
    }

    #[test]
    fn step_can_execute_when_dependencies_met() {
        let mut plan = PlanState::new("p1".into(), "Test".into());
        plan.add_step("Step 1".into(), vec![]);
        plan.add_step("Step 2".into(), vec![0]); // depends on step 0
        plan.approve();

        // Step 0 can execute immediately
        assert!(plan.can_execute(0));
        // Step 1 cannot execute until step 0 completes
        assert!(!plan.can_execute(1));

        // Complete step 0
        plan.update_step_status(0, PlanStepStatus::Completed);
        assert!(plan.can_execute(1));
    }

    #[test]
    fn next_executable_step_returns_first_available() {
        let mut plan = PlanState::new("p1".into(), "Test".into());
        plan.add_step("Step 1".into(), vec![]);
        plan.add_step("Step 2".into(), vec![]);
        plan.approve();

        assert_eq!(plan.next_executable_step(), Some(0));

        plan.update_step_status(0, PlanStepStatus::Completed);
        assert_eq!(plan.next_executable_step(), Some(1));
    }

    #[test]
    fn plan_complete_when_all_steps_done() {
        let mut plan = PlanState::new("p1".into(), "Test".into());
        plan.add_step("Step 1".into(), vec![]);
        plan.add_step("Step 2".into(), vec![]);
        plan.approve();

        assert!(!plan.all_steps_complete());

        plan.update_step_status(0, PlanStepStatus::Completed);
        assert!(!plan.all_steps_complete());

        plan.update_step_status(1, PlanStepStatus::Completed);
        assert!(plan.all_steps_complete());
    }
}
