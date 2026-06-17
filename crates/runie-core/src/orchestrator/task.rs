use serde::{Deserialize, Serialize};
use std::fmt;

use super::TaskStatus;

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

/// A single task assigned to a subagent in the Orchestrator workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Current lifecycle status (includes output/error payload when terminal).
    pub status: TaskStatus,
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
        }
    }

    /// Whether this task is currently executable (Pending and not blocked).
    pub fn is_runnable(&self) -> bool {
        self.status == TaskStatus::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SubagentTask ────────────────────────────────────────────────────────

    #[test]
    fn subagent_task_builder() {
        let task = SubagentTask::new(
            "t1",
            "You are a researcher.",
            "Find all TODOs",
            ModelTrait::Fast,
        );
        assert_eq!(task.id, "t1");
        assert_eq!(task.role_prompt, "You are a researcher.");
        assert_eq!(task.task_description, "Find all TODOs");
        assert_eq!(task.model_trait, ModelTrait::Fast);
        assert_eq!(task.status, TaskStatus::Pending);
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
            status: TaskStatus::Done {
                output: Some("result".into()),
            },
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
}
