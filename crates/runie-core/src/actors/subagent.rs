//! Subagent actor — isolated execution context for a single `SubagentTask`.
//!
//! Each `SubagentActor` is spawned with a `SubagentContext` that contains only
//! the role prompt, task description, allowed tools, and a sanitized project
//! snapshot. The actor does **not** see the full Orchestrator plan or other
//! subagent outputs.

use crate::actor::{spawn_actor, Actor, ActorHandle};
use crate::orchestrator::{ModelTrait, OrchestratorPlan, SubagentTask, TaskStatus};
use crate::tool::{ToolRegistry, builtin_registry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

// ─────────────────────────────────────────────────────────────────────────────
// SubagentContext — the isolated data given to each subagent
// ─────────────────────────────────────────────────────────────────────────────

/// Immutable context handed to a subagent at spawn time.
///
/// Derived from `OrchestratorPlan.tasks[n]` but stripped of all other task
/// data, tool registries are pre-filtered, and project snapshot is sanitized.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentContext {
    /// ID matching the originating `SubagentTask.id`.
    pub task_id: String,
    /// Role description shown to the subagent.
    pub role_prompt: String,
    /// Specific task instruction for this subagent.
    pub task_description: String,
    /// Pre-filtered tool registry containing only allowed tools.
    /// Serialized as a list of tool names; reconstructed at runtime.
    pub allowed_tools: Vec<String>,
    /// Model trait required for this task.
    pub model_trait: ModelTrait,
    /// Sanitized project snapshot (no secrets, no full session history).
    pub project_snapshot: ProjectSnapshot,
}

impl SubagentContext {
    /// Construct from a `SubagentTask` and the originating `OrchestratorPlan`.
    ///
    /// The plan is **not** stored — the subagent must not access other tasks.
    pub fn from_task(task: &SubagentTask, _plan: &OrchestratorPlan) -> Self {
        Self {
            task_id: task.id.clone(),
            role_prompt: task.role_prompt.clone(),
            task_description: task.task_description.clone(),
            allowed_tools: task.tool_filter.clone().unwrap_or_default(),
            model_trait: task.model_trait,
            project_snapshot: ProjectSnapshot::default(),
        }
    }

    /// Build a filtered `ToolRegistry` from this context's allowed tool list.
    ///
    /// If `allowed_tools` is empty, all built-in tools are available.
    pub fn filtered_registry(&self) -> ToolRegistry {
        let all = builtin_registry();
        if self.allowed_tools.is_empty() {
            return all;
        }
        all.filtered(&self.allowed_tools)
    }
}

/// Minimal project snapshot visible to a subagent.
///
/// Contains only public, non-sensitive project metadata. Full file access
/// must go through the `read_file` / `grep` / etc. tools with permission
/// checks in place.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    /// Working directory for this session.
    pub working_dir: PathBuf,
    /// Visible directory tree roots (e.g. `["src", "tests"]`).
    pub visible_roots: Vec<PathBuf>,
    /// Detected project type for context.
    pub project_type: Option<String>,
    /// Git branch, if inside a git repo.
    pub git_branch: Option<String>,
    /// Environment variables visible to the subagent (redacted).
    pub env_vars: HashMap<String, String>,
}

impl ProjectSnapshot {
    /// Construct from the current environment, redacting sensitive values.
    pub fn from_env(working_dir: PathBuf) -> Self {
        let env_vars: HashMap<String, String> = std::env::vars()
            .filter(|(k, _)| !is_sensitive_env_key(k))
            .collect();
        Self {
            working_dir,
            visible_roots: vec![],
            project_type: None,
            git_branch: None,
            env_vars,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Commands & Events
// ─────────────────────────────────────────────────────────────────────────────

/// Command sent to a `SubagentActor`.
#[derive(Debug, Clone)]
pub enum SubagentCommand {
    /// Start executing the subagent task.
    Run,
    /// Report a status update back to the Orchestrator.
    ReportStatus(SubagentStatus),
    /// Request cancellation.
    Cancel,
    /// Update the task status (used by orchestrator for telemetry).
    UpdateStatus(TaskStatus),
}

/// Status of a running subagent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubagentStatus {
    /// Task has been dispatched but not yet started.
    Pending,
    /// Task is actively running.
    Running,
    /// Task completed successfully with output.
    Done { output: String },
    /// Task failed with an error message.
    Failed { error: String },
}

/// Event emitted by a `SubagentActor` to the shared bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubagentEvent {
    /// Subagent started execution.
    Started { task_id: String },
    /// Subagent sent a status update.
    StatusChanged { task_id: String, status: SubagentStatus },
    /// Subagent completed successfully.
    Completed { task_id: String, output: String },
    /// Subagent failed with an error.
    Failed { task_id: String, error: String },
    /// Subagent was cancelled.
    Cancelled { task_id: String },
}

// ─────────────────────────────────────────────────────────────────────────────
// SubagentActor
// ─────────────────────────────────────────────────────────────────────────────

/// Actor that executes a single `SubagentTask` in an isolated context.
///
/// Each instance is spawned with a `SubagentContext` that provides only
/// the task-specific role prompt, description, and pre-filtered tool registry.
/// The actor cannot access the Orchestrator's internal state or other subagents.
#[derive(Debug)]
pub struct SubagentActor {
    /// Isolated context for this subagent.
    pub ctx: SubagentContext,
    /// Current execution status.
    status: SubagentStatus,
    /// Output captured during execution.
    output: Option<String>,
}

impl SubagentActor {
    /// Create a new `SubagentActor` from a task and the originating plan.
    pub fn new(task: &SubagentTask, plan: &OrchestratorPlan) -> Self {
        Self {
            ctx: SubagentContext::from_task(task, plan),
            status: SubagentStatus::Pending,
            output: None,
        }
    }

    /// Create from a raw `SubagentContext`.
    pub fn with_context(ctx: SubagentContext) -> Self {
        Self {
            ctx,
            status: SubagentStatus::Pending,
            output: None,
        }
    }

    /// Spawn this actor and return a `(tx, handle)` pair.
    ///
    /// The caller (typically `OrchestratorActor`) keeps `tx` to send commands
    /// and drops it to signal shutdown.
    pub fn spawn(
        self,
        bus: crate::bus::EventBus<SubagentEvent>,
    ) -> (mpsc::Sender<SubagentCommand>, ActorHandle) {
        spawn_actor(self, bus)
    }

    fn transition_to(&mut self, status: SubagentStatus) {
        self.status = status;
    }

    /// Collect execution output after the subagent loop completes.
    pub fn collect_output(&self) -> Option<String> {
        self.output.clone()
    }
}

impl Actor for SubagentActor {
    type Msg = SubagentCommand;
    type Event = SubagentEvent;

    fn run_body(
        self,
        mut rx: mpsc::Receiver<Self::Msg>,
        bus: crate::bus::EventBus<Self::Event>,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        let task_id = self.ctx.task_id.clone();
        async move {
            // Build isolated tool registry from context
            let registry = self.ctx.filtered_registry();

            // Emit started event
            bus.publish(SubagentEvent::Started {
                task_id: task_id.clone(),
            });

            // Process commands
            while let Some(cmd) = rx.recv().await {
                match handle_subagent_command(&task_id, cmd, &mut Default::default(), &bus) {
                    ControlFlow::Continue => {}
                    ControlFlow::Break => break,
                }
            }

            // When channel closes, the orchestrator has dropped us — normal exit.
            let _ = (registry, task_id);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Command handlers (small, lint-compliant functions)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlFlow {
    Continue,
    Break,
}

fn handle_subagent_command(
    task_id: &str,
    cmd: SubagentCommand,
    _state: &mut (),
    bus: &crate::bus::EventBus<SubagentEvent>,
) -> ControlFlow {
    match cmd {
        SubagentCommand::Run => {
            bus.publish(SubagentEvent::StatusChanged {
                task_id: task_id.to_string(),
                status: SubagentStatus::Running,
            });
            ControlFlow::Continue
        }
        SubagentCommand::ReportStatus(status) => {
            bus.publish(SubagentEvent::StatusChanged {
                task_id: task_id.to_string(),
                status,
            });
            ControlFlow::Continue
        }
        SubagentCommand::Cancel => {
            bus.publish(SubagentEvent::Cancelled {
                task_id: task_id.to_string(),
            });
            ControlFlow::Break
        }
        SubagentCommand::UpdateStatus(_) => ControlFlow::Continue,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ToolRegistry filtered() method
// ─────────────────────────────────────────────────────────────────────────────

impl ToolRegistry {
    /// Return a new `ToolRegistry` containing only the named tools.
    ///
    /// - If `allowed` is empty, all built-in tools are included (no filter).
    /// - Unknown tool names are silently skipped.
    pub fn filtered(&self, allowed: &[String]) -> ToolRegistry {
        if allowed.is_empty() {
            return self.clone();
        }
        let allowed_set: std::collections::HashSet<_> = allowed.iter().collect();
        let mut out = ToolRegistry::new();
        for (name, tool) in &self.tools {
            if allowed_set.contains(name) {
                out.tools.insert(name.clone(), Arc::clone(tool));
            }
        }
        out
    }
}

// We need Clone for ToolRegistry
impl Clone for ToolRegistry {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
        }
    }
}

// We need is_sensitive_env_key in permissions
fn is_sensitive_env_key(key: &str) -> bool {
    let upper = key.to_uppercase();
    upper.contains("SECRET")
        || upper.contains("PASSWORD")
        || upper.contains("PRIVATE_KEY")
        || upper.contains("AWS_ACCESS_KEY")
        || upper.contains("GITHUB_TOKEN")
        || upper.contains("API_KEY")
        || upper.contains("DATABASE_URL")
        || upper.starts_with("RUNIE_")
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plan() -> OrchestratorPlan {
        OrchestratorPlan {
            tasks: vec![
                SubagentTask::new("t1", "You are a code reviewer.", "Review src/main.rs", ModelTrait::General),
                SubagentTask::new("t2", "You are a test writer.", "Write tests for src/main.rs", ModelTrait::General),
            ],
            synthesis_trait: ModelTrait::General,
            summary: Some("Review and test".to_string()),
            rationale: None,
        }
    }

    fn all_registry() -> ToolRegistry {
        builtin_registry()
    }

    #[test]
    fn subagent_context_hides_orchestrator_plan() {
        let plan = sample_plan();
        let task = &plan.tasks[0];
        let ctx = SubagentContext::from_task(task, &plan);
        assert_eq!(ctx.task_id, "t1");
        assert_eq!(ctx.task_description, "Review src/main.rs");
        assert_eq!(ctx.role_prompt, "You are a code reviewer.");
        assert_eq!(ctx.model_trait, ModelTrait::General);
        // The subagent context does not expose the full plan or other tasks
        assert!(ctx.allowed_tools.is_empty()); // task has no tool_filter
    }

    #[test]
    fn subagent_context_respects_tool_filter() {
        let mut plan = sample_plan();
        plan.tasks[0].tool_filter = Some(vec!["read_file".to_string(), "grep".to_string()]);
        let task = &plan.tasks[0];
        let ctx = SubagentContext::from_task(task, &plan);
        assert_eq!(ctx.allowed_tools, vec!["read_file", "grep"]);
    }

    #[test]
    fn tool_filter_limits_registry() {
        let registry = all_registry();
        let filtered = registry.filtered(&["read_file".to_string(), "grep".to_string()]);
        assert!(filtered.get("read_file").is_some());
        assert!(filtered.get("grep").is_some());
        assert!(filtered.get("bash").is_none());
        assert!(filtered.get("write_file").is_none());
    }

    #[test]
    fn tool_filter_empty_allows_all() {
        let registry = all_registry();
        let filtered = registry.filtered(&[]);
        // Empty filter means no restriction — all tools available
        assert!(filtered.get("bash").is_some());
        assert!(filtered.get("read_file").is_some());
    }

    #[test]
    fn tool_filter_unknown_names_skipped() {
        let registry = all_registry();
        let filtered = registry.filtered(&["read_file".to_string(), "nonexistent".to_string()]);
        assert!(filtered.get("read_file").is_some());
        assert!(filtered.get("nonexistent").is_none());
    }

    #[test]
    fn subagent_status_serialization() {
        let status = SubagentStatus::Done {
            output: "test output".to_string(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("done"));
        let roundtrip: SubagentStatus = serde_json::from_str(&json).unwrap();
        assert!(matches!(roundtrip, SubagentStatus::Done { output } if output == "test output"));
    }

    #[test]
    fn subagent_context_serialization() {
        let ctx = SubagentContext {
            task_id: "t1".to_string(),
            role_prompt: "reviewer".to_string(),
            task_description: "review code".to_string(),
            allowed_tools: vec!["read_file".to_string()],
            model_trait: ModelTrait::General,
            project_snapshot: ProjectSnapshot::default(),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let roundtrip: SubagentContext = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.task_id, "t1");
        assert_eq!(roundtrip.allowed_tools, vec!["read_file"]);
    }

    #[test]
    fn project_snapshot_from_env() {
        let snap = ProjectSnapshot::from_env(std::env::current_dir().unwrap());
        // Sensitive keys must be filtered
        for key in snap.env_vars.keys() {
            assert!(!is_sensitive_env_key(key), "sensitive key leaked: {key}");
        }
    }

    #[test]
    fn sensitive_env_keys_identified() {
        let sensitive = [
            "AWS_SECRET_ACCESS_KEY",
            "GITHUB_TOKEN",
            "API_KEY",
            "RUNIE_API_KEY",
            "MY_PASSWORD",
        ];
        for key in sensitive {
            assert!(
                is_sensitive_env_key(key),
                "should be sensitive: {key}"
            );
        }
        let safe = ["PATH", "HOME", "USER", "TERM", "EDITOR"];
        for key in safe {
            assert!(
                !is_sensitive_env_key(key),
                "should not be sensitive: {key}"
            );
        }
    }
}
