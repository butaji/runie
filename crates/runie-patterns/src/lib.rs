//! Orchestration patterns for multi-agent workflows (see `PATTERNS.md`).
//!
//! A [`Pattern`] executes a user request through one of three strategies:
//! `single` (one agent end to end), `swarm` (leader + fan-out workers), or
//! `eval-optimizer` (evaluate / revise loop). Phase 1 shipped the core
//! types and [`SinglePattern`]; Phase 2 added [`SwarmPattern`] (parallel +
//! delegation variants); Phase 3 adds the swarm dag variant (backed by
//! [`primitives::dag::Dag`]) and [`EvalOptimizerPattern`].
//!
//! # Deviation from PATTERNS.md: `WorkerRunner` instead of `LeaderHandle`
//!
//! The design doc's `Context` carries `LeaderHandle` / `SessionState` from
//! runie-core. Those don't fit here: `LeaderAgentHandle::run` is
//! fire-and-forget and cannot return a worker's final text. Instead, this
//! crate defines the [`WorkerRunner`] trait — "run one agent turn, return the
//! final assistant text" — which runie-tui implements over
//! `runie_agent::subagent::run_subagent`. Tests mock it. This keeps
//! runie-patterns free of any runie-core dependency.
//!
//! # Cancellation contract
//!
//! Every pattern must honor [`Context::abort`]:
//! - On abort, in-flight worker runs are dropped (via `tokio::select!`).
//! - Partial results from already-completed workers are preserved.
//! - The pattern returns `TerminationReason::Error("aborted")`.
//! - Clean shutdown with no zombie tasks.

mod eval_optimizer;
pub mod primitives;
mod single;
mod swarm;

pub use eval_optimizer::EvalOptimizerPattern;
pub use single::SinglePattern;
pub use swarm::{SwarmPattern, SwarmVariant};

use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

/// Why a task/pattern ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationReason {
    Completed,
    MaxRoundsReached,
    Timeout,
    Error(String),
    Approved,
}

/// One event in an agent's trace timeline.
#[derive(Debug, Clone, PartialEq)]
pub enum TraceEvent {
    Handoff { from: String, to: String },
    ToolCall { tool: String, duration_ms: u64 },
    Error { error: String },
    Termination { reason: TerminationReason },
}

/// Trace of a single agent run.
#[derive(Debug, Clone)]
pub struct AgentTrace {
    pub agent_id: String,
    /// Human-readable label for feed rows (e.g. the worker's task text).
    pub description: String,
    /// Final output text (or failure message) — shown when the row is expanded.
    pub output: String,
    pub start_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub events: Vec<TraceEvent>,
}

/// Channel for streaming agent traces to observers (e.g. the TUI).
pub type TraceSender = mpsc::UnboundedSender<AgentTrace>;

/// Effective pattern configuration (mirrors the `[mode]` config section;
/// constructed by the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternConfig {
    /// "single" | "swarm" | "eval-optimizer".
    pub active: String,
    /// Max parallel workers.
    pub workers: usize,
    /// Max iterations.
    pub max_rounds: usize,
    /// Per-task timeout.
    pub timeout_ms: u64,
    /// Retries per task on failure.
    pub max_retries: u32,
    /// Consecutive failures before fail-fast.
    pub circuit_breaker: u32,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            active: "single".into(),
            workers: 3,
            max_rounds: 5,
            timeout_ms: 120_000,
            max_retries: 2,
            circuit_breaker: 3,
        }
    }
}

/// A unit of work for one worker agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerTask {
    pub id: String,
    pub prompt: String,
    pub provider: String,
    pub model: String,
    pub read_only: bool,
}

/// Executes one agent turn and returns the final assistant text.
///
/// Implemented in runie-tui over `runie_agent::subagent::run_subagent`;
/// mocked in tests.
#[async_trait::async_trait]
pub trait WorkerRunner: Send + Sync {
    async fn run(&self, task: WorkerTask) -> anyhow::Result<String>;
}

/// Shared execution context handed to every pattern.
pub struct Context {
    pub config: PatternConfig,
    /// (provider, model) priority list: leader = first; workers reuse the
    /// leader model when fewer models than workers are configured.
    pub models: Vec<(String, String)>,
    pub semaphore: Arc<Semaphore>,
    pub trace_tx: TraceSender,
    pub abort: CancellationToken,
    pub runner: Arc<dyn WorkerRunner>,
}

/// Result of a pattern execution.
#[derive(Debug, Clone)]
pub struct PatternOutput {
    pub result: String,
    pub termination: TerminationReason,
    pub traces: Vec<AgentTrace>,
}

/// An orchestration strategy executable against a [`Context`].
#[async_trait::async_trait]
pub trait Pattern: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &str;
    async fn execute(&self, ctx: &Context, input: &str) -> anyhow::Result<PatternOutput>;
}

/// Registry of available patterns, keyed by [`Pattern::name`].
pub struct PatternRegistry {
    patterns: Vec<Box<dyn Pattern>>,
}

impl PatternRegistry {
    /// Empty registry; register patterns manually.
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    pub fn register(&mut self, pattern: Box<dyn Pattern>) {
        self.patterns.push(pattern);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Pattern> {
        self.patterns
            .iter()
            .find(|p| p.name() == name)
            .map(|p| &**p)
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.patterns.iter().map(|p| p.name()).collect()
    }
}

impl Default for PatternRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(SinglePattern));
        registry.register(Box::new(SwarmPattern::parallel()));
        registry.register(Box::new(EvalOptimizerPattern));
        registry
    }
}

/// Model for the agent at `worker_index` in the priority list.
///
/// The leader (index 0) always uses the first model; workers reuse the leader
/// model when fewer models than workers are configured (PATTERNS.md "Model
/// Fallback" — no error, proceed with available models).
pub fn model_for(models: &[(String, String)], worker_index: usize) -> (String, String) {
    models
        .get(worker_index)
        .or_else(|| models.first())
        .cloned()
        .unwrap_or_default()
}
