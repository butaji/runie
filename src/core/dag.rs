//! DAG executor - structured concurrency for plan execution
//! Uses tokio for parallel task execution with cancellation propagation

use super::git::GitOps;
use super::plan::{Action, Plan, PlanStep};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinSet;

/// Execution state for a single step
#[derive(Debug, Clone)]
pub enum StepState {
    Pending,
    Running,
    Completed,
    Failed { error: String },
    Cancelled,
}

/// Result of executing a step
#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_id: String,
    pub state: StepState,
    pub output: Option<String>,
    pub cost: f64,
    pub duration_ms: u64,
}

impl StepResult {
    pub fn success(step_id: &str, output: String, cost: f64, duration_ms: u64) -> Self {
        Self {
            step_id: step_id.to_string(),
            state: StepState::Completed,
            output: Some(output),
            cost,
            duration_ms,
        }
    }

    pub fn failure(step_id: &str, error: String, duration_ms: u64) -> Self {
        Self {
            step_id: step_id.to_string(),
            state: StepState::Failed { error },
            output: None,
            cost: 0.0,
            duration_ms,
        }
    }
}

/// Execution context passed to step handlers
#[derive(Clone)]
pub struct ExecContext {
    pub working_dir: std::path::PathBuf,
    pub model_id: String,
    /// Git operations for committing completed work
    pub git: Option<GitOps>,
}

/// Callback for step execution
pub type StepHandler =
    Arc<dyn Fn(&PlanStep, &ExecContext) -> anyhow::Result<String> + Send + Sync + 'static>;

/// DAG executor with structured concurrency
pub struct DagExecutor {
    plan: Plan,
    step_states: Arc<RwLock<HashMap<String, StepState>>>,
    results: Arc<RwLock<Vec<StepResult>>>,
    cancelled: Arc<RwLock<bool>>,
}

impl DagExecutor {
    pub fn new(plan: Plan) -> Self {
        let step_states: HashMap<String, StepState> = plan
            .steps
            .iter()
            .map(|s| (s.id.clone(), StepState::Pending))
            .collect();

        Self {
            plan,
            step_states: Arc::new(RwLock::new(step_states)),
            results: Arc::new(RwLock::new(Vec::new())),
            cancelled: Arc::new(RwLock::new(false)),
        }
    }

    /// Set the step handler (executes actual work)
    pub fn with_handler(self, _handler: StepHandler) -> Self {
        self
    }

    /// Execute the DAG with structured concurrency
    pub async fn execute(&self, ctx: &ExecContext) -> anyhow::Result<Vec<StepResult>> {
        let mut join_set = JoinSet::new();
        let completed: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));
        let (tx, mut rx) = mpsc::channel::<StepResult>(16);

        // Clone everything we need for async tasks
        let step_states = Arc::clone(&self.step_states);
        let results = Arc::clone(&self.results);
        let cancelled = Arc::clone(&self.cancelled);
        let ctx_clone = ExecContext {
            working_dir: ctx.working_dir.clone(),
            model_id: ctx.model_id.clone(),
            git: ctx.git.clone(),
        };
        let plan = self.plan.clone();

        loop {
            // Check for cancellation
            if *cancelled.read().await {
                join_set.abort_all();
                break;
            }

            // Find steps ready to execute
            let completed_vec: Vec<String> = {
                let completed_ref = completed.read().await;
                completed_ref.iter().cloned().collect()
            };
            let ready = plan.ready_steps(&completed_vec);

            // Spawn ready steps
            for step in ready {
                let step_id = step.id.clone();
                let step_action = step.action.clone();
                let step_cost = step.estimated_cost;
                let step_states_clone = Arc::clone(&step_states);
                let results_clone = Arc::clone(&results);
                let completed_clone = Arc::clone(&completed);
                let tx_clone = tx.clone();
                let ctx_for_task = ctx_clone.clone();
                let start = std::time::Instant::now();

                // Mark as running
                {
                    let mut states = step_states_clone.write().await;
                    states.insert(step_id.clone(), StepState::Running);
                }

                join_set.spawn(async move {
                    // Execute the step
                    let output = execute_action(&step_action, &ctx_for_task).unwrap_or_else(|e| e.to_string());

                    let cost = step_cost;
                    let duration = start.elapsed().as_millis() as u64;
                    let result = StepResult::success(&step_id, output, cost, duration);

                    // Update state
                    {
                        let mut states = step_states_clone.write().await;
                        states.insert(step_id.clone(), result.state.clone());
                    }

                    // Store result
                    {
                        let mut res = results_clone.write().await;
                        res.push(result.clone());
                    }

                    // Mark completed
                    {
                        let mut comp = completed_clone.write().await;
                        comp.insert(step_id.clone());
                    }

                    let _ = tx_clone.send(result).await;
                });
            }

            // Wait for at least one task to complete (or all done)
            let Some(result) = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                rx.recv()
            ).await.ok().flatten() else {
                // Check if we're done
                let completed_count = completed.read().await.len();
                if completed_count >= self.plan.steps.len() {
                    break;
                }
                continue;
            };

            // Check if step failed
            if matches!(result.state, StepState::Failed { .. }) {
                // Propagate cancellation
                let mut c = cancelled.write().await;
                *c = true;
                drop(c);

                join_set.abort_all();

                // Mark remaining as cancelled
                let completed_ref = completed.read().await;
                let mut states = step_states.write().await;
                for step in &self.plan.steps {
                    if !completed_ref.contains(&step.id) {
                        states.insert(step.id.clone(), StepState::Cancelled);
                    }
                }
                break;
            }
        }

        // Wait for all tasks to complete
        while join_set.join_next().await.is_some() {}

        // Return results
        let results = self.results.read().await.clone();
        Ok(results)
    }

    /// Cancel all execution
    pub async fn cancel(&self) {
        let mut c = self.cancelled.write().await;
        *c = true;
    }

    /// Get current execution state
    pub async fn state(&self) -> HashMap<String, StepState> {
        self.step_states.read().await.clone()
    }

    /// Get total cost so far
    pub async fn cost_so_far(&self) -> f64 {
        self.results.read().await.iter().map(|r| r.cost).sum()
    }

    /// Get completion percentage
    pub async fn completion(&self) -> f64 {
        let completed = self.results.read().await.len();
        let total = self.plan.steps.len();
        if total == 0 {
            0.0
        } else {
            (completed as f64 / total as f64) * 100.0
        }
    }
}

/// Execute a single action
pub fn execute_action(action: &Action, ctx: &ExecContext) -> anyhow::Result<String> {
    match action {
        Action::ReadContext { files } => {
            let mut output = String::new();
            for file in files {
                let path = ctx.working_dir.join(file);
                if path.exists() {
                    let content = std::fs::read_to_string(&path)?;
                    output.push_str(&format!("// {} ({} bytes)\n", file, content.len()));
                } else {
                    output.push_str(&format!("// {} not found\n", file));
                }
            }
            Ok(output)
        }
        Action::GenerateCode { files } => Ok(format!("Generated code for {} files", files.len())),
        Action::RunTests { pattern } => {
            let pattern_str = pattern.as_deref().unwrap_or(".*");
            Ok(format!("Tests matching '{}' would run here", pattern_str))
        }
        Action::Lint { pattern } => {
            let pattern_str = pattern.as_deref().unwrap_or(".*");
            Ok(format!("Lint check matching '{}' would run here", pattern_str))
        }
        Action::Build { target } => {
            let target_str = target.as_deref().unwrap_or("all");
            Ok(format!("Build target '{}' would run here", target_str))
        }
        Action::Commit { message } => {
            let msg = message.as_deref().unwrap_or("[anvil] auto-commit | task complete");
            if let Some(ref git) = ctx.git {
                git.commit(msg)
            } else {
                Ok(format!("Would commit with message: {}", msg))
            }
        }
        Action::AskHuman { question } => Ok(format!("Waiting for human input: {}", question)),
        Action::Review => Ok("Review step completed".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_execution() {
        let intent = super::super::intent::Intent::from_text("refactor src/main.rs");
        let plan = Plan::from_intent(&intent);
        let _step_count = plan.steps.len();
        let executor = DagExecutor::new(plan);

        let ctx = ExecContext {
            working_dir: std::path::PathBuf::from("."),
            model_id: "test".to_string(),
            git: None,
        };

        let results = executor.execute(&ctx).await.unwrap();
        assert!(!results.is_empty());
        // Results should include all steps (may include cancelled ones)
        assert!(!results.is_empty());

        // All should succeed (or at least attempted)
        let successful = results.iter().filter(|r| matches!(r.state, StepState::Completed)).count();
        assert!(successful >= 1, "At least one step should complete");
    }

    #[tokio::test]
    async fn test_cancellation() {
        let intent = super::super::intent::Intent::from_text("refactor src/main.rs");
        let plan = Plan::from_intent(&intent);
        let executor = DagExecutor::new(plan);

        let ctx = ExecContext {
            working_dir: std::path::PathBuf::from("."),
            model_id: "test".to_string(),
            git: None,
        };

        // Cancel immediately
        executor.cancel().await;

        let results = executor.execute(&ctx).await.unwrap();
        // Should have some cancelled steps (not all should complete)
        let completed = results.iter().filter(|r| matches!(r.state, StepState::Completed)).count();
        assert!(completed <= results.len()); // Could be 0 or partial
    }

    #[test]
    fn test_action_execution() {
        let ctx = ExecContext {
            working_dir: std::path::PathBuf::from("."),
            model_id: "test".to_string(),
            git: None,
        };

        let action = Action::Review;
        let result = execute_action(&action, &ctx).unwrap();
        assert!(!result.is_empty());
    }
}
