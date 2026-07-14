//! Swarm pattern: coordinated multi-agent work (PATTERNS.md Phase 2).
//!
//! A leader agent plans, worker agents execute the planned tasks concurrently
//! (bounded by `Context::semaphore`), and the leader synthesizes the worker
//! outputs into a final answer. Two variants share one engine:
//!
//! - [`SwarmVariant::Parallel`] — exactly one plan → execute → synthesize cycle.
//! - [`SwarmVariant::Delegation`] — repeat plan → execute up to
//!   `max_rounds`; the leader finishes early by returning an empty/invalid
//!   plan. Each plan prompt after round 1 carries a summary of prior rounds.
//!
//! # Cancellation contract
//!
//! On `Context::abort` the execute join is abandoned via `tokio::select!`.
//! Dropping the join detaches any in-flight worker spawns: their runner calls
//! keep running in the background but their results and late traces are
//! discarded. The pattern returns `TerminationReason::Error("aborted")` with
//! the traces collected so far.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use futures::future::join_all;
use tokio::sync::Semaphore;

use crate::{
    model_for, AgentTrace, Context, Pattern, PatternConfig, PatternOutput, TerminationReason,
    TraceEvent, TraceSender, WorkerRunner, WorkerTask,
};

/// Per-worker output truncation in the synthesis prompt (~4000 chars).
const SYNTHESIS_OUTPUT_CHARS: usize = 4000;
/// Per-result truncation in the prior-rounds summary.
const SUMMARY_OUTPUT_CHARS: usize = 500;

/// Swarm execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmVariant {
    /// Fan-out: one plan → execute → synthesize cycle.
    Parallel,
    /// Leader delegates over multiple rounds until done or `max_rounds`.
    Delegation,
}

/// Coordinated multi-agent swarm: leader plans, workers execute, leader
/// synthesizes. Both variants report [`Pattern::name`] as `"swarm"`.
pub struct SwarmPattern {
    variant: SwarmVariant,
}

impl SwarmPattern {
    /// Fan-out variant: a single plan → execute → synthesize cycle.
    pub fn parallel() -> Self {
        Self {
            variant: SwarmVariant::Parallel,
        }
    }

    /// Delegation variant: leader assigns tasks over up to `max_rounds` rounds.
    pub fn delegation() -> Self {
        Self {
            variant: SwarmVariant::Delegation,
        }
    }

    /// The configured execution variant.
    pub fn variant(&self) -> &SwarmVariant {
        &self.variant
    }

    /// Run the plan → execute loop; the caller handles synthesis.
    async fn run_rounds(&self, ctx: &Context, state: &Arc<SwarmState>, input: &str) -> LoopOutcome {
        let max_rounds = match self.variant {
            SwarmVariant::Parallel => 1,
            SwarmVariant::Delegation => ctx.config.max_rounds.max(1),
        };
        let mut outcome = LoopOutcome::default();
        let mut prior_summary = String::new();

        for round in 1..=max_rounds {
            if ctx.abort.is_cancelled() {
                outcome.aborted = true;
                return outcome;
            }
            match plan_round(ctx, state, &self.variant, input, round, &prior_summary).await {
                PlanSignal::Aborted => {
                    outcome.aborted = true;
                    return outcome;
                }
                // Delegation only: the leader signals "done" with an
                // empty/invalid plan on a later round.
                PlanSignal::Done => return outcome,
                PlanSignal::Tasks(tasks) => {
                    let round_outcome = execute_round(ctx, state, round, tasks).await;
                    if round_outcome.aborted {
                        outcome.aborted = true;
                        return outcome;
                    }
                    prior_summary = summarize_round(round, &round_outcome);
                    outcome.successes.extend(round_outcome.successes);
                    outcome.errors.extend(round_outcome.errors);
                    if state.is_tripped() {
                        return outcome;
                    }
                    if round == max_rounds && self.variant == SwarmVariant::Delegation {
                        outcome.termination = TerminationReason::MaxRoundsReached;
                    }
                }
            }
        }
        outcome
    }
}

#[async_trait::async_trait]
impl Pattern for SwarmPattern {
    fn name(&self) -> &'static str {
        "swarm"
    }

    fn description(&self) -> &str {
        "Coordinated multi-agent swarm — leader plans, workers execute in parallel, leader synthesizes"
    }

    async fn execute(&self, ctx: &Context, input: &str) -> anyhow::Result<PatternOutput> {
        let state = Arc::new(SwarmState::default());
        if ctx.abort.is_cancelled() {
            return Ok(finish(&state, String::new(), aborted()));
        }

        let rounds = self.run_rounds(ctx, &state, input).await;
        if rounds.aborted {
            return Ok(finish(&state, String::new(), aborted()));
        }
        if state.is_tripped() {
            let message = format!(
                "circuit breaker tripped after {} consecutive failures",
                ctx.config.circuit_breaker
            );
            return Ok(finish(&state, rounds.errors.join("\n"), TerminationReason::Error(message)));
        }
        if rounds.successes.is_empty() {
            let joined = rounds.errors.join("\n");
            return Ok(finish(
                &state,
                joined.clone(),
                TerminationReason::Error(format!("all workers failed: {joined}")),
            ));
        }

        match synthesize(ctx, &state, input, &rounds.successes).await {
            LeaderCall::Aborted => Ok(finish(&state, String::new(), aborted())),
            LeaderCall::Failed(message) => Ok(finish(
                &state,
                message.clone(),
                TerminationReason::Error(format!("synthesis failed: {message}")),
            )),
            LeaderCall::Text(result) => Ok(finish(&state, result, rounds.termination)),
        }
    }
}

fn aborted() -> TerminationReason {
    TerminationReason::Error("aborted".into())
}

fn finish(state: &SwarmState, result: String, termination: TerminationReason) -> PatternOutput {
    PatternOutput {
        result,
        termination,
        traces: state.traces.lock().unwrap().clone(),
    }
}

/// Shared mutable state across all agents of one swarm execution.
#[derive(Default)]
struct SwarmState {
    traces: Mutex<Vec<AgentTrace>>,
    consecutive_failures: AtomicU32,
    tripped: AtomicBool,
}

impl SwarmState {
    fn finish_trace(&self, trace_tx: &TraceSender, trace: AgentTrace) {
        // Observers may have gone away; a failed send must not fail the run.
        let _ = trace_tx.send(trace.clone());
        self.traces.lock().unwrap().push(trace);
    }

    fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
    }

    fn record_failure(&self, circuit_breaker: u32) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
        if failures >= circuit_breaker {
            self.tripped.store(true, Ordering::SeqCst);
        }
    }

    fn is_tripped(&self) -> bool {
        self.tripped.load(Ordering::SeqCst)
    }
}

/// Outcome of one leader runner call (plan or synthesis).
enum LeaderCall {
    Text(String),
    Failed(String),
    Aborted,
}

/// What the leader decided in a plan round.
enum PlanSignal {
    Tasks(Vec<String>),
    /// Delegation: leader returned an empty/invalid plan on a later round.
    Done,
    Aborted,
}

/// Aggregated result of the plan → execute loop.
struct LoopOutcome {
    successes: Vec<(String, String)>,
    errors: Vec<String>,
    termination: TerminationReason,
    aborted: bool,
}

impl Default for LoopOutcome {
    fn default() -> Self {
        Self {
            successes: Vec::new(),
            errors: Vec::new(),
            termination: TerminationReason::Completed,
            aborted: false,
        }
    }
}

/// Result of one execute round.
#[derive(Default)]
struct RoundOutcome {
    successes: Vec<(String, String)>,
    errors: Vec<String>,
    aborted: bool,
}

/// Per-worker execution environment — cheap to clone into spawned tasks.
#[derive(Clone)]
struct WorkerEnv {
    runner: Arc<dyn WorkerRunner>,
    semaphore: Arc<Semaphore>,
    trace_tx: TraceSender,
    config: PatternConfig,
    models: Vec<(String, String)>,
    state: Arc<SwarmState>,
}

/// One leader runner call with abort race and trace recording.
async fn call_leader(ctx: &Context, state: &SwarmState, id: &str, prompt: String) -> LeaderCall {
    let (provider, model) = model_for(&ctx.models, 0);
    let task = WorkerTask {
        id: id.to_string(),
        prompt,
        provider,
        model,
        read_only: true,
    };
    let start_time = Utc::now();
    let start = Instant::now();

    let run = ctx.runner.run(task);
    tokio::pin!(run);
    let mut events = Vec::new();
    let call = tokio::select! {
        () = ctx.abort.cancelled() => {
            events.push(TraceEvent::Termination { reason: aborted() });
            LeaderCall::Aborted
        }
        outcome = &mut run => match outcome {
            Ok(text) => {
                events.push(TraceEvent::Termination { reason: TerminationReason::Completed });
                LeaderCall::Text(text)
            }
            Err(error) => {
                let message = error.to_string();
                events.push(TraceEvent::Error { error: message.clone() });
                events.push(TraceEvent::Termination {
                    reason: TerminationReason::Error(message.clone()),
                });
                LeaderCall::Failed(message)
            }
        },
    };

    state.finish_trace(
        &ctx.trace_tx,
        AgentTrace {
            agent_id: id.to_string(),
            description: id.to_string(),
            output: match &call {
                LeaderCall::Text(text) => text.clone(),
                LeaderCall::Failed(message) => message.clone(),
                LeaderCall::Aborted => String::new(),
            },
            start_time,
            duration_ms: start.elapsed().as_millis() as u64,
            events,
        },
    );
    call
}

/// Plan round: ask the leader for the task list. Round 1 falls back to the
/// original input as a single task when the plan is missing/unparseable; on
/// later delegation rounds that means "done".
async fn plan_round(
    ctx: &Context,
    state: &SwarmState,
    variant: &SwarmVariant,
    input: &str,
    round: usize,
    prior_summary: &str,
) -> PlanSignal {
    let prompt = build_plan_prompt(variant, ctx.config.workers, input, round, prior_summary);
    let id = format!("leader-plan-{round}");
    let tasks = match call_leader(ctx, state, &id, prompt).await {
        LeaderCall::Aborted => return PlanSignal::Aborted,
        LeaderCall::Failed(_) => None,
        LeaderCall::Text(response) => parse_tasks(&response),
    };
    match tasks {
        Some(tasks) => PlanSignal::Tasks(tasks),
        None if round == 1 => PlanSignal::Tasks(vec![input.to_string()]),
        None => PlanSignal::Done,
    }
}

/// Execute round: run all planned tasks concurrently (bounded by the
/// semaphore), collecting successes and errors.
async fn execute_round(
    ctx: &Context,
    state: &Arc<SwarmState>,
    round: usize,
    tasks: Vec<String>,
) -> RoundOutcome {
    let env = WorkerEnv {
        runner: Arc::clone(&ctx.runner),
        semaphore: Arc::clone(&ctx.semaphore),
        trace_tx: ctx.trace_tx.clone(),
        config: ctx.config.clone(),
        models: ctx.models.clone(),
        state: Arc::clone(state),
    };
    let handles: Vec<_> = tasks
        .into_iter()
        .enumerate()
        .map(|(index, task_text)| tokio::spawn(run_worker(env.clone(), round, index, task_text)))
        .collect();

    // On abort the join is dropped, detaching in-flight worker spawns; their
    // late results/traces are discarded (see module docs).
    let results = tokio::select! {
        () = ctx.abort.cancelled() => return RoundOutcome { aborted: true, ..RoundOutcome::default() },
        results = join_all(handles) => results,
    };

    let mut outcome = RoundOutcome::default();
    for result in results {
        match result {
            Ok(Ok(success)) => outcome.successes.push(success),
            Ok(Err(error)) => outcome.errors.push(error),
            Err(join_error) => outcome
                .errors
                .push(format!("worker task failed to join: {join_error}")),
        }
    }
    outcome
}

/// Run one worker task with per-task timeout and retries, feeding the
/// circuit breaker. Returns `(worker_id, output)` on success.
async fn run_worker(
    env: WorkerEnv,
    round: usize,
    index: usize,
    task_text: String,
) -> Result<(String, String), String> {
    let worker_id = format!("worker-{round}-{index}");
    let permit = env
        .semaphore
        .acquire()
        .await
        .map_err(|e| format!("{worker_id}: semaphore closed: {e}"))?;

    // Queued tasks are skipped once the circuit breaker has tripped —
    // "stop dispatching new tasks"; already-running tasks finish.
    if env.state.is_tripped() {
        return Err(format!("{worker_id}: skipped (circuit breaker tripped)"));
    }

    let start_time = Utc::now();
    let start = Instant::now();
    let mut events = vec![TraceEvent::Handoff {
        from: "leader".into(),
        to: worker_id.clone(),
    }];

    let (provider, model) = model_for(&env.models, index + 1);
    let task_description = task_text.clone();
    let task = WorkerTask {
        id: worker_id.clone(),
        prompt: task_text,
        provider,
        model,
        read_only: false,
    };
    let timeout = Duration::from_millis(env.config.timeout_ms);

    let mut last_error = String::new();
    let mut output = None;
    for _ in 0..=env.config.max_retries {
        match tokio::time::timeout(timeout, env.runner.run(task.clone())).await {
            Ok(Ok(text)) => {
                output = Some(text);
                break;
            }
            Ok(Err(error)) => last_error = error.to_string(),
            Err(_) => last_error = format!("timed out after {} ms", env.config.timeout_ms),
        }
        events.push(TraceEvent::Error {
            error: last_error.clone(),
        });
    }
    drop(permit);

    let result = match output {
        Some(text) => {
            env.state.record_success();
            events.push(TraceEvent::Termination {
                reason: TerminationReason::Completed,
            });
            Ok((worker_id.clone(), text))
        }
        None => {
            env.state.record_failure(env.config.circuit_breaker);
            let message = format!(
                "{worker_id}: failed after {} attempts: {last_error}",
                env.config.max_retries + 1
            );
            events.push(TraceEvent::Termination {
                reason: TerminationReason::Error(message.clone()),
            });
            Err(message)
        }
    };

    env.state.finish_trace(
        &env.trace_tx,
        AgentTrace {
            agent_id: worker_id,
            description: task_description,
            output: match &result {
                Ok((_, text)) => text.clone(),
                Err(message) => message.clone(),
            },
            start_time,
            duration_ms: start.elapsed().as_millis() as u64,
            events,
        },
    );
    result
}

/// Synthesis round: the leader consolidates all worker outputs.
async fn synthesize(
    ctx: &Context,
    state: &SwarmState,
    input: &str,
    successes: &[(String, String)],
) -> LeaderCall {
    call_leader(ctx, state, "leader-synthesize", build_synthesis_prompt(input, successes)).await
}

/// Extract a JSON array of task strings from a leader response.
///
/// Lenient: scans from the first `[` to the last `]`, accepting a plain
/// string array or objects with a `task`/`prompt` string field. Returns
/// `None` when nothing usable is found (no array, empty array, unparseable).
fn parse_tasks(response: &str) -> Option<Vec<String>> {
    let start = response.find('[')?;
    let end = response.rfind(']')?;
    if end <= start {
        return None;
    }
    let slice = &response[start..=end];
    if let Ok(tasks) = serde_json::from_str::<Vec<String>>(slice) {
        return non_empty(tasks);
    }
    let values: Vec<serde_json::Value> = serde_json::from_str(slice).ok()?;
    non_empty(values.iter().filter_map(task_from_value).collect())
}

fn non_empty(tasks: Vec<String>) -> Option<Vec<String>> {
    if tasks.is_empty() {
        None
    } else {
        Some(tasks)
    }
}

fn task_from_value(value: &serde_json::Value) -> Option<String> {
    let field = value.get("task").or_else(|| value.get("prompt"))?;
    field.as_str().map(str::to_string)
}

fn build_plan_prompt(
    variant: &SwarmVariant,
    workers: usize,
    input: &str,
    round: usize,
    prior_summary: &str,
) -> String {
    let (marker, instruction) = match variant {
        SwarmVariant::Parallel => (
            "[swarm-plan parallel]",
            format!(
                "Decompose the task below into 2 to {workers} independent subtasks that can be executed in parallel."
            ),
        ),
        SwarmVariant::Delegation => (
            "[swarm-plan delegation]",
            "Assign the task below to specialist workers, with clear instructions per task."
                .to_string(),
        ),
    };
    let mut prompt = format!(
        "{marker}\nYou are the leader of a coordinated worker swarm. {instruction}\n\
         Reply ONLY with a JSON array of strings, one per task. No prose, no markdown fences.\n\n\
         Task:\n{input}"
    );
    if round > 1 && !prior_summary.is_empty() {
        prompt.push_str(&format!("\n\nPrevious rounds:\n{prior_summary}"));
    }
    prompt
}

fn build_synthesis_prompt(input: &str, successes: &[(String, String)]) -> String {
    let mut prompt = format!("[swarm-synthesize]\nOriginal task:\n{input}\n\nWorker outputs:\n");
    for (i, (worker_id, output)) in successes.iter().enumerate() {
        prompt.push_str(&format!(
            "{}. [{}] {}\n",
            i + 1,
            worker_id,
            truncate(output, SYNTHESIS_OUTPUT_CHARS)
        ));
    }
    prompt.push_str("\nProduce the final consolidated answer to the original task.");
    prompt
}

fn summarize_round(round: usize, outcome: &RoundOutcome) -> String {
    let mut summary = format!("Round {round}:\n");
    for (worker_id, output) in &outcome.successes {
        summary.push_str(&format!(
            "- {worker_id} (ok): {}\n",
            truncate(output, SUMMARY_OUTPUT_CHARS)
        ));
    }
    for error in &outcome.errors {
        summary.push_str(&format!("- {error}\n"));
    }
    summary
}

/// Truncate to at most `max_chars` characters (on char boundaries).
fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        text.chars().take(max_chars).collect()
    }
}
