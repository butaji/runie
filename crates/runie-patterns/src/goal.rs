//! Goal Pattern — structured goal execution with planning, execution, and verification.
//!
//! A GoalPattern decomposes a user goal into a task list and executes each task
//! with iterative verification. This differs from SwarmPattern by focusing on
//! a single coherent goal with checkpoint tracking rather than parallel delegation.
//!
//! # Lifecycle
//!
//! 1. **Planning**: Decompose goal into numbered tasks
//! 2. **Executing**: Work through tasks sequentially
//! 3. **Verifying**: Check each task's completion
//! 4. **Complete**: Goal achieved or budget exhausted

use chrono::Utc;

use crate::{
    AgentTrace, Context, Pattern, PatternOutput, TerminationReason, TraceEvent, WorkerTask,
};

/// Default max rounds for goal pattern.
#[allow(dead_code)]
const DEFAULT_MAX_ROUNDS: usize = 10;

/// Goal pattern execution state.
#[derive(Debug, Default)]
pub struct GoalPatternState {
    /// Current task index.
    pub current_task: usize,
    /// Total tasks in the plan.
    pub total_tasks: usize,
    /// Task descriptions.
    pub tasks: Vec<String>,
    /// Completed task indices.
    pub completed: Vec<usize>,
}

/// Goal Pattern — structured multi-phase goal execution.
pub struct GoalPattern {
    /// Execution state.
    state: GoalPatternState,
}

impl Default for GoalPattern {
    fn default() -> Self {
        Self::new()
    }
}

impl GoalPattern {
    /// Create a new goal pattern.
    pub fn new() -> Self {
        Self { state: GoalPatternState::default() }
    }

    /// Create with initial tasks.
    pub fn with_tasks(tasks: Vec<String>) -> Self {
        let total_tasks = tasks.len();
        Self { state: GoalPatternState { current_task: 0, total_tasks, tasks, completed: Vec::new() } }
    }

    /// Get current task description.
    pub fn current_task_desc(&self) -> Option<&str> {
        self.state.tasks.get(self.state.current_task).map(|s| s.as_str())
    }

    /// Check if all tasks are complete.
    pub fn is_complete(&self) -> bool {
        self.state.current_task >= self.state.total_tasks
    }

    /// Progress to next task.
    pub fn advance(&mut self) {
        if self.state.current_task < self.state.total_tasks {
            self.state.completed.push(self.state.current_task);
            self.state.current_task += 1;
        }
    }

    /// Get completion percentage.
    pub fn completion_pct(&self) -> f32 {
        if self.state.total_tasks == 0 {
            1.0
        } else {
            self.state.completed.len() as f32 / self.state.total_tasks as f32
        }
    }

    /// Parse tasks from structured text (numbered list).
    pub fn parse_tasks(text: &str) -> Vec<String> {
        text.lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                // Match numbered items like "1. Task" or "1) Task"
                let task = trimmed
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ')
                    .trim();
                if task.is_empty() {
                    None
                } else {
                    Some(task.to_string())
                }
            })
            .collect()
    }
}

#[async_trait::async_trait]
#[allow(clippy::too_many_lines)]
impl Pattern for GoalPattern {
    fn name(&self) -> &'static str {
        "goal"
    }

    fn description(&self) -> &str {
        "Structured goal execution with planning and verification"
    }

    async fn execute(&self, ctx: &Context, input: &str) -> anyhow::Result<PatternOutput> {
        let start = chrono::Utc::now();
        let mut traces = Vec::new();

        // Phase 1: Planning
        let (tasks, tasks_text) = match Self::run_planning_phase(ctx, input).await {
            Ok((t, txt)) => (t, txt),
            Err(e) => {
                return Ok(PatternOutput {
                    result: format!("Planning failed: {}", e),
                    termination: TerminationReason::Error(e.to_string()),
                    traces,
                    circuit_breaker_tripped: false,
                });
            }
        };

        if tasks.is_empty() {
            return Ok(PatternOutput {
                result: "No tasks generated from goal".to_string(),
                termination: TerminationReason::Error("no tasks".to_string()),
                traces,
                circuit_breaker_tripped: false,
            });
        }

        traces.push(Self::make_planning_trace(start, &tasks_text));

        // Phase 2: Execute tasks
        let results = Self::run_execution_phase(ctx, &tasks, &mut traces).await;

        // Phase 3: Verification
        let (verification, verify_duration_ms) =
            Self::run_verification_phase(ctx, input, &results).await;

        traces.push(AgentTrace {
            agent_id: "verifier".to_string(),
            description: "Goal verifier".to_string(),
            output: verification.clone(),
            start_time: chrono::Utc::now(),
            duration_ms: verify_duration_ms,
            events: vec![],
        });

        let termination = if verification.to_lowercase().contains("yes") {
            TerminationReason::Completed
        } else {
            TerminationReason::MaxRoundsReached
        };

        Ok(PatternOutput {
            result: format!("Goal execution complete.\n\nTasks:\n{}\n\nVerification:\n{}",
                           results.join("\n\n"), verification),
            termination,
            traces,
            circuit_breaker_tripped: false,
        })
    }
}

impl GoalPattern {
    /// Phase 1: run the planner agent and parse its output into tasks.
    async fn run_planning_phase(
        ctx: &Context,
        input: &str,
    ) -> anyhow::Result<(Vec<String>, String)> {
        let prompt = format!(
            "Decompose this goal into numbered tasks:\n\n{}\n\n\
             Format: 1. Task description\n        2. Task description\n\
             Keep tasks focused and actionable.",
            input
        );
        let (provider, model) = ctx.models.first()
            .map(|p| (p.0.clone(), p.1.clone()))
            .unwrap_or_default();
        let text = ctx.runner.run(WorkerTask {
            id: uuid::Uuid::new_v4().to_string(),
            prompt,
            provider,
            model,
            read_only: false,
        }).await?;
        let tasks = Self::parse_tasks(&text);
        Ok((tasks, text))
    }

    /// Build the planning trace event.
    fn make_planning_trace(start: chrono::DateTime<Utc>, tasks_text: &str) -> AgentTrace {
        AgentTrace {
            agent_id: "planner".to_string(),
            description: "Goal planner".to_string(),
            output: tasks_text.to_string(),
            start_time: start,
            duration_ms: 0,
            events: vec![TraceEvent::Termination { reason: TerminationReason::Completed }],
        }
    }

    /// Phase 2: execute each planned task and collect results and traces.
    async fn run_execution_phase(
        ctx: &Context,
        tasks: &[String],
        traces: &mut Vec<AgentTrace>,
    ) -> Vec<String> {
        let (provider, model) = ctx.models.first()
            .map(|p| (p.0.clone(), p.1.clone()))
            .unwrap_or_default();
        let max_rounds = ctx.config.max_rounds.min(tasks.len());
        let mut results = Vec::new();

        for round in 0..max_rounds {
            if ctx.abort.is_cancelled() {
                break;
            }
            let task_desc = &tasks[round % tasks.len()];
            let out = Self::execute_one_task(
                ctx, &provider, &model, round, task_desc, traces,
            ).await;
            if let Some(output) = out {
                results.push(output);
            }
        }
        results
    }

    /// Execute one task and push its trace. Returns formatted output on success.
    async fn execute_one_task(
        ctx: &Context,
        provider: &str,
        model: &str,
        round: usize,
        task_desc: &str,
        traces: &mut Vec<AgentTrace>,
    ) -> Option<String> {
        let task_start = Utc::now();
        let out = ctx.runner.run(WorkerTask {
            id: uuid::Uuid::new_v4().to_string(),
            prompt: format!("Execute this task:\n\n{}", task_desc),
            provider: provider.to_string(),
            model: model.to_string(),
            read_only: false,
        }).await;
        let duration_ms = (Utc::now() - task_start).num_milliseconds() as u64;

        match out {
            Ok(output) => {
                let result = format!("Task {}: {}\n{}", round + 1, task_desc, output);
                traces.push(AgentTrace {
                    agent_id: format!("task-{}", round + 1),
                    description: task_desc.to_string(),
                    output,
                    start_time: task_start,
                    duration_ms,
                    events: vec![TraceEvent::Termination { reason: TerminationReason::Completed }],
                });
                Some(result)
            }
            Err(e) => {
                traces.push(AgentTrace {
                    agent_id: format!("task-{}", round + 1),
                    description: task_desc.to_string(),
                    output: e.to_string(),
                    start_time: task_start,
                    duration_ms,
                    events: vec![TraceEvent::Error { error: e.to_string() }],
                });
                None
            }
        }
    }

    /// Phase 3: run the verifier agent and return its output and duration.
    async fn run_verification_phase(
        ctx: &Context,
        input: &str,
        results: &[String],
    ) -> (String, u64) {
        let (provider, model) = ctx.models.first()
            .map(|p| (p.0.clone(), p.1.clone()))
            .unwrap_or_default();
        let verify_prompt = format!(
            "Based on the following task executions, verify if the original goal was achieved:\n\n\
             Goal: {}\n\nResults:\n{}\n\n\
             Was the goal successfully achieved? Answer yes or no and explain.",
            input,
            results.join("\n---\n")
        );
        let verify_start = Utc::now();
        let out = ctx.runner.run(WorkerTask {
            id: uuid::Uuid::new_v4().to_string(),
            prompt: verify_prompt,
            provider,
            model,
            read_only: true,
        }).await;
        let verify_duration_ms = (Utc::now() - verify_start).num_milliseconds() as u64;
        let verification = match out {
            Ok(v) => v,
            Err(e) => format!("Verification inconclusive: {}", e),
        };
        (verification, verify_duration_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_numbered_tasks() {
        let text = "1. First task\n2. Second task\n3. Third task";
        let tasks = GoalPattern::parse_tasks(text);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0], "First task");
        assert_eq!(tasks[1], "Second task");
    }

    #[test]
    fn parse_tasks_with_dash() {
        let text = "1. First task\n2) Second task\n3. Third task";
        let tasks = GoalPattern::parse_tasks(text);
        assert_eq!(tasks.len(), 3);
    }

    #[test]
    fn parse_empty_tasks() {
        let text = "";
        let tasks = GoalPattern::parse_tasks(text);
        assert!(tasks.is_empty());
    }

    #[test]
    fn goal_pattern_default() {
        let pattern = GoalPattern::default();
        assert_eq!(pattern.state.current_task, 0);
        assert_eq!(pattern.state.total_tasks, 0);
        assert!(pattern.is_complete());
    }

    #[test]
    fn goal_pattern_with_tasks() {
        let tasks = vec!["Task 1".to_string(), "Task 2".to_string()];
        let pattern = GoalPattern::with_tasks(tasks.clone());
        assert_eq!(pattern.state.total_tasks, 2);
        assert_eq!(pattern.current_task_desc(), Some("Task 1"));
        assert!(!pattern.is_complete());
    }

    #[test]
    fn goal_pattern_advance() {
        let tasks = vec!["Task 1".to_string(), "Task 2".to_string()];
        let mut pattern = GoalPattern::with_tasks(tasks);
        assert_eq!(pattern.current_task_desc(), Some("Task 1"));
        pattern.advance();
        assert_eq!(pattern.current_task_desc(), Some("Task 2"));
        assert!(!pattern.is_complete());
        pattern.advance();
        assert!(pattern.is_complete());
    }

    #[test]
    fn goal_pattern_completion_pct() {
        let tasks = vec!["T1".to_string(), "T2".to_string(), "T3".to_string(), "T4".to_string()];
        let mut pattern = GoalPattern::with_tasks(tasks);
        assert_eq!(pattern.completion_pct(), 0.0);
        pattern.advance();
        assert_eq!(pattern.completion_pct(), 0.25);
        pattern.advance();
        assert_eq!(pattern.completion_pct(), 0.5);
        pattern.advance();
        assert_eq!(pattern.completion_pct(), 0.75);
        pattern.advance();
        assert_eq!(pattern.completion_pct(), 1.0);
    }

    #[test]
    fn pattern_name() {
        let pattern = GoalPattern::new();
        assert_eq!(pattern.name(), "goal");
    }
}
