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
use std::fmt;

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

/// Model traits used by the Orchestrator to express subagent requirements.
/// The harness resolves these to concrete (provider, model) pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTrait {
    /// Fast, low-cost model for straightforward tasks (e.g. file ops, formatting).
    Fast,
    /// General-purpose capable model for most tasks.
    General,
    /// Model with reasoning / chain-of-thought for planning and analysis.
    Reasoning,
    /// Vision-capable model for tasks involving images or screenshots.
    Vision,
    /// Model with long context for large-file analysis.
    LongContext,
}

impl ModelTrait {
    /// Human-readable label for display in the UI.
    pub fn label(&self) -> &'static str {
        match self {
            ModelTrait::Fast => "fast",
            ModelTrait::General => "general",
            ModelTrait::Reasoning => "reasoning",
            ModelTrait::Vision => "vision",
            ModelTrait::LongContext => "long-context",
        }
    }

    /// Short display label for compact UI contexts (e.g. sidebar badges).
    pub fn short_label(&self) -> &'static str {
        match self {
            ModelTrait::Fast => "⚡",
            ModelTrait::General => "◆",
            ModelTrait::Reasoning => "🧠",
            ModelTrait::Vision => "👁",
            ModelTrait::LongContext => "📄",
        }
    }
}

impl fmt::Display for ModelTrait {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Task status
// ─────────────────────────────────────────────────────────────────────────────

/// Lifecycle state of a single subagent task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is queued but not yet started.
    Pending,
    /// Task is currently executing.
    Running,
    /// Task is waiting for user input/approval before continuing.
    AwaitingUser,
    /// Task completed successfully.
    Done,
    /// Task failed with an error.
    Failed,
}

impl TaskStatus {
    /// Whether a status transition from `self` to `next` is valid.
    pub fn can_transition_to(&self, next: TaskStatus) -> bool {
        use TaskStatus::*;
        match (self, next) {
            // Pending can start running
            (Pending, Running) => true,
            // Running can await user, complete, or fail
            (Running, AwaitingUser) | (Running, Done) | (Running, Failed) => true,
            // AwaitingUser can resume running or fail
            (AwaitingUser, Running) | (AwaitingUser, Failed) => true,
            // Done and Failed are terminal — no transitions
            (Done, _) | (Failed, _) => false,
            // Same state is always allowed
            (a, b) if *a == b => true,
            // All other transitions are invalid
            _ => false,
        }
    }

    /// Human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::AwaitingUser => "awaiting",
            TaskStatus::Done => "done",
            TaskStatus::Failed => "failed",
        }
    }

    /// Whether this status is terminal (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Done | TaskStatus::Failed)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Subagent task
// ─────────────────────────────────────────────────────────────────────────────

/// A single task assigned to a subagent in the Orchestrator workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentTask {
    /// Unique identifier for this task within the plan.
    pub id: String,
    /// Role description shown to the subagent (e.g. "You are a code reviewer.").
    pub role_prompt: String,
    /// Specific task instruction for this subagent.
    pub task_description: String,
    /// Optional list of tool names this subagent is allowed to use.
    /// `None` means all available tools are permitted.
    pub tool_filter: Option<Vec<String>>,
    /// Model trait required for this task (harness resolves to concrete model).
    pub model_trait: ModelTrait,
    /// Current lifecycle status.
    pub status: TaskStatus,
    /// Output produced by the subagent (populated on Done).
    pub output: Option<String>,
}

impl SubagentTask {
    /// Construct a new pending task.
    pub fn new(
        id: impl Into<String>,
        role_prompt: impl Into<String>,
        task_description: impl Into<String>,
        model_trait: ModelTrait,
    ) -> Self {
        Self {
            id: id.into(),
            role_prompt: role_prompt.into(),
            task_description: task_description.into(),
            tool_filter: None,
            model_trait,
            status: TaskStatus::Pending,
            output: None,
        }
    }

    /// Whether this task is currently executable (Pending and not blocked).
    pub fn is_runnable(&self) -> bool {
        self.status == TaskStatus::Pending
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Orchestrator plan
// ─────────────────────────────────────────────────────────────────────────────

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
    pub fn simple(
        task_description: impl Into<String>,
        model_trait: ModelTrait,
    ) -> Self {
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
        self.tasks
            .iter()
            .filter(|t| t.status.is_terminal())
            .count()
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

// ─────────────────────────────────────────────────────────────────────────────
// Plan result
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// Orchestrator context
// ─────────────────────────────────────────────────────────────────────────────

/// A single entry in the Orchestrator's dialogue log with the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueEntry {
    /// The Orchestrator asked a question via `ask_user`.
    Question(String),
    /// The user's answer to the preceding question.
    Answer(String),
}

/// Working memory for the Orchestrator during Team mode planning.
///
/// Accumulates questions sent via `ask_user` and their answers so the plan
/// can be refined in one shot before submission.
#[derive(Debug, Clone, Default)]
pub struct OrchestratorContext {
    /// Chronological log of questions and answers.
    dialogue: Vec<DialogueEntry>,
}

impl OrchestratorContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self { dialogue: Vec::new() }
    }

    /// Record a question that was sent to the user.
    pub fn record_question(&mut self, question: impl Into<String>) {
        self.dialogue.push(DialogueEntry::Question(question.into()));
    }

    /// Record the user's answer to the most recent pending question.
    pub fn record_answer(&mut self, answer: impl Into<String>) {
        self.dialogue.push(DialogueEntry::Answer(answer.into()));
    }

    /// All dialogue entries.
    pub fn dialogue(&self) -> &[DialogueEntry] {
        &self.dialogue
    }

    /// Questions that have been asked but not yet answered.
    pub fn pending_questions(&self) -> Vec<&str> {
        // Scan forward: every Question that has no subsequent Answer yet is pending.
        let mut pending = Vec::new();
        for entry in &self.dialogue {
            match entry {
                DialogueEntry::Question(q) => pending.push(q.as_str()),
                DialogueEntry::Answer(_) => {
                    if !pending.is_empty() {
                        pending.remove(pending.len() - 1);
                    }
                }
            }
        }
        pending
    }

    /// Whether there are any unanswered questions.
    pub fn has_pending_questions(&self) -> bool {
        !self.pending_questions().is_empty()
    }

    /// Whether the dialogue log is empty.
    pub fn is_empty(&self) -> bool {
        self.dialogue.is_empty()
    }

    /// Number of entries in the dialogue log.
    pub fn len(&self) -> usize {
        self.dialogue.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── TaskStatus transitions ──────────────────────────────────────────────

    #[test]
    fn task_status_transitions_valid() {
        assert!(
            TaskStatus::Pending.can_transition_to(TaskStatus::Running),
            "Pending → Running must be valid"
        );
        assert!(
            TaskStatus::Running.can_transition_to(TaskStatus::Done),
            "Running → Done must be valid"
        );
        assert!(
            TaskStatus::Running.can_transition_to(TaskStatus::AwaitingUser),
            "Running → AwaitingUser must be valid"
        );
        assert!(
            TaskStatus::Running.can_transition_to(TaskStatus::Failed),
            "Running → Failed must be valid"
        );
        assert!(
            TaskStatus::AwaitingUser.can_transition_to(TaskStatus::Running),
            "AwaitingUser → Running must be valid"
        );
    }

    #[test]
    fn task_status_transitions_invalid() {
        assert!(
            !TaskStatus::Done.can_transition_to(TaskStatus::Pending),
            "Done → Pending must be invalid"
        );
        assert!(
            !TaskStatus::Failed.can_transition_to(TaskStatus::Running),
            "Failed → Running must be invalid"
        );
        assert!(
            !TaskStatus::Pending.can_transition_to(TaskStatus::Done),
            "Pending → Done must be invalid (must go through Running)"
        );
        assert!(
            !TaskStatus::Pending.can_transition_to(TaskStatus::AwaitingUser),
            "Pending → AwaitingUser must be invalid (must go through Running)"
        );
    }

    #[test]
    fn task_status_is_terminal() {
        assert!(TaskStatus::Done.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(!TaskStatus::AwaitingUser.is_terminal());
    }

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
                output: None,
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
        assert!(decoded.tasks[0].output.is_none());
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

        plan.tasks[0].status = TaskStatus::Done;
        assert!(plan.is_complete());
    }

    // ── SubagentTask ────────────────────────────────────────────────────────

    #[test]
    fn subagent_task_builder() {
        let task = SubagentTask::new("t1", "You are a researcher.", "Find all TODOs", ModelTrait::Fast);
        assert_eq!(task.id, "t1");
        assert_eq!(task.role_prompt, "You are a researcher.");
        assert_eq!(task.task_description, "Find all TODOs");
        assert_eq!(task.model_trait, ModelTrait::Fast);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.output.is_none());
        assert!(task.tool_filter.is_none());
        assert!(task.is_runnable());
    }

    #[test]
    fn subagent_task_not_runnable_when_not_pending() {
        let task = SubagentTask::new("t1", "role", "task", ModelTrait::General);
        assert!(task.is_runnable());
        let done = SubagentTask {
            id: task.id,
            role_prompt: task.role_prompt,
            task_description: task.task_description,
            tool_filter: task.tool_filter,
            model_trait: task.model_trait,
            status: TaskStatus::Done,
            output: Some("result".into()),
        };
        assert!(!done.is_runnable());
    }

    // ── ModelTrait ─────────────────────────────────────────────────────────

    #[test]
    fn model_trait_labels() {
        assert_eq!(ModelTrait::Fast.label(), "fast");
        assert_eq!(ModelTrait::General.label(), "general");
        assert_eq!(ModelTrait::Reasoning.label(), "reasoning");
        assert_eq!(ModelTrait::Vision.label(), "vision");
        assert_eq!(ModelTrait::LongContext.label(), "long-context");
    }

    #[test]
    fn model_trait_display() {
        assert_eq!(ModelTrait::Fast.to_string(), "fast");
        assert_eq!(ModelTrait::Reasoning.to_string(), "reasoning");
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

    // ── OrchestratorContext ────────────────────────────────────────────────

    #[test]
    fn orchestrator_context_records_dialogue() {
        let mut ctx = OrchestratorContext::new();
        assert!(ctx.dialogue().is_empty());
        ctx.record_question("Which file?");
        assert_eq!(ctx.dialogue().len(), 1);
        assert!(matches!(ctx.dialogue()[0], DialogueEntry::Question(_)));
        ctx.record_answer("src/lib.rs");
        assert_eq!(ctx.dialogue().len(), 2);
        assert!(matches!(ctx.dialogue()[1], DialogueEntry::Answer(_)));
    }

    #[test]
    fn orchestrator_context_pending_questions() {
        let mut ctx = OrchestratorContext::new();
        assert!(ctx.pending_questions().is_empty());
        ctx.record_question("Scope?");
        ctx.record_question("Priority?");
        assert_eq!(ctx.pending_questions().len(), 2);
        ctx.record_answer("Large");
        assert_eq!(ctx.pending_questions().len(), 1); // second still pending
        ctx.record_answer("High");
        assert!(ctx.pending_questions().is_empty());
    }

    #[test]
    fn orchestrator_context_has_pending() {
        let mut ctx = OrchestratorContext::new();
        assert!(!ctx.has_pending_questions());
        ctx.record_question("Q1");
        assert!(ctx.has_pending_questions());
    }
}
