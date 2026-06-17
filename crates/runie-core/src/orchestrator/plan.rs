use serde::{Deserialize, Serialize};

use super::{ModelTrait, SubagentTask, TaskStatus};

/// A complete Orchestrator plan for Team mode execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratorPlan {
    /// Ordered list of subagent tasks to execute.
    pub tasks: Vec<SubagentTask>,
    /// Model trait used for the synthesis step (producing the final response).
    pub synthesis_trait: ModelTrait,
    /// Optional user-facing summary of the planned workflow.
    pub summary: Option<String>,
    /// Optional human-readable plan rationale (shown before execution).
    pub rationale: Option<String>,
}

impl OrchestratorPlan {
    /// Build a plan with a single task and general-purpose synthesis.
    pub fn simple(task_description: impl Into<String>, model_trait: ModelTrait) -> Self {
        Self {
            tasks: vec![SubagentTask::new(
                "main",
                "You are a coding assistant.",
                task_description,
                model_trait,
            )],
            synthesis_trait: ModelTrait::General,
            summary: None,
            rationale: None,
        }
    }

    /// Number of tasks in the plan.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Number of tasks that have reached a terminal state.
    pub fn completed_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.status.is_terminal()).count()
    }

    /// Whether all tasks have reached a terminal state.
    pub fn is_complete(&self) -> bool {
        !self.tasks.is_empty() && self.tasks.iter().all(|t| t.status.is_terminal())
    }

    /// Whether all tasks are still Pending.
    pub fn is_unstarted(&self) -> bool {
        self.tasks.iter().all(|t| t.status == TaskStatus::Pending)
    }
}

/// Outcome of executing an `OrchestratorPlan`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanResult {
    /// Whether the plan succeeded (all tasks Done, not Failed).
    pub success: bool,
    /// Final synthesized response assembled from subagent outputs.
    pub response: String,
    /// List of task IDs that failed, with their error messages.
    pub failures: Vec<TaskFailure>,
    /// Elapsed time in seconds.
    pub elapsed_secs: f64,
}

/// Description of a single task failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFailure {
    pub task_id: String,
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── OrchestratorPlan round-trip ────────────────────────────────────────

    #[test]
    fn plan_serializes_round_trip() {
        let plan = OrchestratorPlan {
            tasks: vec![SubagentTask {
                id: "review-1".into(),
                role_prompt: "You are a code reviewer.".into(),
                task_description: "Review src/main.rs for bugs.".into(),
                tool_filter: Some(vec!["Read".into(), "Bash".into()]),
                model_trait: ModelTrait::Reasoning,
                status: TaskStatus::Pending,
            }],
            synthesis_trait: ModelTrait::General,
            summary: Some("Review then synthesize".into()),
            rationale: Some("Parallelize review across files".into()),
        };

        let json = serde_json::to_string(&plan).unwrap();
        let decoded: OrchestratorPlan = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.tasks.len(), 1);
        assert_eq!(decoded.tasks[0].id, "review-1");
        assert_eq!(decoded.tasks[0].role_prompt, "You are a code reviewer.");
        assert_eq!(decoded.tasks[0].model_trait, ModelTrait::Reasoning);
        assert_eq!(decoded.tasks[0].status, TaskStatus::Pending);
        assert_eq!(decoded.synthesis_trait, ModelTrait::General);
        assert_eq!(decoded.summary.as_deref(), Some("Review then synthesize"));
    }

    #[test]
    fn plan_simple_helper() {
        let plan = OrchestratorPlan::simple("Fix the bug", ModelTrait::Fast);
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].id, "main");
        assert_eq!(plan.tasks[0].status, TaskStatus::Pending);
        assert_eq!(plan.synthesis_trait, ModelTrait::General);
    }

    #[test]
    fn plan_completion_helpers() {
        let mut plan = OrchestratorPlan::simple("task", ModelTrait::General);
        assert!(plan.is_unstarted());
        assert!(!plan.is_complete());

        plan.tasks[0].status = TaskStatus::Running;
        assert!(!plan.is_unstarted());
        assert!(!plan.is_complete());

        plan.tasks[0].status = TaskStatus::Done { output: None };
        assert!(plan.is_complete());
    }

    // ── PlanResult ─────────────────────────────────────────────────────────

    #[test]
    fn plan_result_serializes() {
        let result = PlanResult {
            success: true,
            response: "All done.".into(),
            failures: vec![],
            elapsed_secs: 12.5,
        };
        let json = serde_json::to_string(&result).unwrap();
        let decoded: PlanResult = serde_json::from_str(&json).unwrap();
        assert!(decoded.success);
        assert_eq!(decoded.response, "All done.");
        assert!(decoded.failures.is_empty());
    }

    #[test]
    fn plan_result_with_failures() {
        let result = PlanResult {
            success: false,
            response: "Partially complete.".into(),
            failures: vec![TaskFailure {
                task_id: "analyze".into(),
                error: "Timeout".into(),
            }],
            elapsed_secs: 30.0,
        };
        let json = serde_json::to_string(&result).unwrap();
        let decoded: PlanResult = serde_json::from_str(&json).unwrap();
        assert!(!decoded.success);
        assert_eq!(decoded.failures.len(), 1);
        assert_eq!(decoded.failures[0].task_id, "analyze");
    }
}
