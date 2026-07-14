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
//! - [`SwarmVariant::Dag`] — one plan produces a dependency graph; workers
//!   execute in topological waves, each task receiving its dependencies'
//!   outputs as context. Tasks with failed dependencies are skipped.
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
use tokio::task::JoinError;

use crate::primitives::dag::{CycleError, Dag};
use crate::{
    model_for, AgentTrace, Context, Pattern, PatternConfig, PatternOutput, TerminationReason,
    TraceEvent, TraceSender, WorkerRunner, WorkerTask,
};

/// Per-worker output truncation in the synthesis prompt (~4000 chars).
const SYNTHESIS_OUTPUT_CHARS: usize = 4000;
/// Per-result truncation in the prior-rounds summary.
const SUMMARY_OUTPUT_CHARS: usize = 500;
/// Per-dependency output truncation in dag worker prompts.
const DEP_OUTPUT_CHARS: usize = 1000;

/// Swarm execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmVariant {
    /// Fan-out: one plan → execute → synthesize cycle.
    Parallel,
    /// Leader delegates over multiple rounds until done or `max_rounds`.
    Delegation,
    /// Workers form a dependency graph executed in topological waves.
    Dag,
}

/// Coordinated multi-agent swarm: leader plans, workers execute, leader
/// synthesizes. All variants report [`Pattern::name`] as `"swarm"`.
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

    /// Dag variant: leader plans a dependency graph; workers execute in
    /// topological waves with dependency outputs as context.
    pub fn dag() -> Self {
        Self {
            variant: SwarmVariant::Dag,
        }
    }

    /// The configured execution variant.
    pub fn variant(&self) -> &SwarmVariant {
        &self.variant
    }

    /// Run the plan → execute loop; the caller handles synthesis.
    async fn run_rounds(&self, ctx: &Context, state: &Arc<SwarmState>, input: &str) -> LoopOutcome {
        let max_rounds = match self.variant {
            SwarmVariant::Parallel | SwarmVariant::Dag => 1,
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

        let rounds = match self.variant {
            SwarmVariant::Parallel | SwarmVariant::Delegation => {
                self.run_rounds(ctx, &state, input).await
            }
            SwarmVariant::Dag => run_dag(ctx, &state, input).await,
        };
        if rounds.aborted {
            return Ok(finish(&state, String::new(), aborted()));
        }
        // A variant-level failure (dag dependency cycle) ends the pattern
        // before any worker ran or synthesis happened.
        if let TerminationReason::Error(message) = &rounds.termination {
            return Ok(finish(&state, message.clone(), rounds.termination.clone()));
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
        .map(|(index, task_text)| tokio::spawn(run_worker(env.clone(), round, index, task_text, None)))
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
/// One worker task with retries, timeout, circuit-breaker accounting, and a
/// completion trace. `description` overrides the trace/feed-row label;
/// defaults to `task_text`. Dag tasks pass the bare task so injected
/// dependency context does not leak into the feed row.
async fn run_worker(
    env: WorkerEnv,
    round: usize,
    index: usize,
    task_text: String,
    description: Option<String>,
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
    let task_description = description.unwrap_or_else(|| task_text.clone());
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
        SwarmVariant::Dag => (
            "[swarm-plan dag]",
            "Decompose the task below into subtasks with explicit dependencies, forming a DAG that executes in waves."
                .to_string(),
        ),
    };
    let format_instruction = match variant {
        SwarmVariant::Dag => "Reply ONLY with a JSON array of objects, one per task, each \
             {\"task\": \"...\", \"deps\": [<zero-based indices of earlier tasks it depends on>]}. \
             No prose, no markdown fences.",
        SwarmVariant::Parallel | SwarmVariant::Delegation => {
            "Reply ONLY with a JSON array of strings, one per task. No prose, no markdown fences."
        }
    };
    let mut prompt = format!(
        "{marker}\nYou are the leader of a coordinated worker swarm. {instruction}\n\
         {format_instruction}\n\n\
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

/// Dag variant: plan once, execute in topological waves (PATTERNS.md Phase 3).
/// A dependency cycle or an aborted/failed leader call ends the pattern
/// before synthesis; an unparseable plan falls back to a single task, like
/// the parallel variant.
async fn run_dag(ctx: &Context, state: &Arc<SwarmState>, input: &str) -> LoopOutcome {
    let mut outcome = LoopOutcome::default();
    let prompt = build_plan_prompt(&SwarmVariant::Dag, ctx.config.workers, input, 1, "");
    let response = match call_leader(ctx, state, "leader-plan-1", prompt).await {
        LeaderCall::Aborted => {
            outcome.aborted = true;
            return outcome;
        }
        LeaderCall::Failed(_) => None,
        LeaderCall::Text(text) => Some(text),
    };

    let (tasks, edges) = response
        .as_deref()
        .and_then(parse_dag_plan)
        .unwrap_or_else(|| (vec![input.to_string()], Vec::new()));

    let waves = match build_waves(ctx, state, &tasks, &edges) {
        Ok(waves) => waves,
        Err(CycleError(node)) => {
            let message = format!("dependency cycle detected at node {node}");
            outcome.errors.push(message.clone());
            outcome.termination = TerminationReason::Error(message);
            return outcome;
        }
    };

    let env = WorkerEnv {
        runner: Arc::clone(&ctx.runner),
        semaphore: Arc::clone(&ctx.semaphore),
        trace_tx: ctx.trace_tx.clone(),
        config: ctx.config.clone(),
        models: ctx.models.clone(),
        state: Arc::clone(state),
    };
    let mut run = DagRun::new(tasks, &edges);
    for (wave_index, wave) in waves.iter().enumerate() {
        if ctx.abort.is_cancelled() {
            outcome.aborted = true;
            return outcome;
        }
        if state.is_tripped() {
            break;
        }
        let wave_outcome = run_wave(ctx, &env, state, &mut run, wave_index + 1, wave).await;
        if wave_outcome.aborted {
            outcome.aborted = true;
            return outcome;
        }
        outcome.successes.extend(wave_outcome.successes);
        outcome.errors.extend(wave_outcome.errors);
    }
    outcome
}

/// A parsed dag plan: task texts plus `(task, dependency)` edges.
type DagPlan = (Vec<String>, Vec<(usize, usize)>);

/// Parse a dag plan: an array of objects with a required `"task"` string and
/// an optional `"deps"` array of zero-based indices of earlier tasks.
///
/// Reuses the lenient array scan of [`parse_tasks`] (first `[` to last `]`).
/// Returns `(tasks, edges)` where an edge `(task, dependency)` means `task`
/// waits for `dependency`. Returns `None` when nothing usable is found —
/// including any object missing a valid `"task"`, since dropping an object
/// would shift the indices `deps` refer to. Non-integer dep entries are
/// ignored; out-of-range deps are dropped later by [`Dag::add_edge`].
fn parse_dag_plan(response: &str) -> Option<DagPlan> {
    let start = response.find('[')?;
    let end = response.rfind(']')?;
    if end <= start {
        return None;
    }
    let slice = &response[start..=end];
    let values: Vec<serde_json::Value> = serde_json::from_str(slice).ok()?;
    if values.is_empty() {
        return None;
    }
    let mut tasks = Vec::new();
    let mut edges = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let task = value.get("task")?.as_str()?.to_string();
        if let Some(deps) = value.get("deps").and_then(|d| d.as_array()) {
            for dep in deps {
                if let Some(dep_index) = dep.as_u64() {
                    edges.push((index, dep_index as usize));
                }
            }
        }
        tasks.push(task);
    }
    Some((tasks, edges))
}

/// Build the [`Dag`] and compute execution waves. Out-of-range dependencies
/// are dropped (via [`Dag::add_edge`]) and logged with a trace Error event
/// instead of failing the plan.
fn build_waves(
    ctx: &Context,
    state: &SwarmState,
    tasks: &[String],
    edges: &[(usize, usize)],
) -> Result<Vec<Vec<usize>>, CycleError> {
    let mut dag = Dag::new();
    for task in tasks {
        dag.add_node(task.clone());
    }
    for &(task, dependency) in edges {
        dag.add_edge(task, dependency);
    }
    let dropped: Vec<(usize, usize)> = edges
        .iter()
        .copied()
        .filter(|&(task, dependency)| task >= tasks.len() || dependency >= tasks.len())
        .collect();
    if !dropped.is_empty() {
        let events: Vec<TraceEvent> = dropped
            .iter()
            .map(|(task, dependency)| TraceEvent::Error {
                error: format!(
                    "dropped out-of-range dependency: task {task} depends on {dependency}"
                ),
            })
            .collect();
        state.finish_trace(
            &ctx.trace_tx,
            AgentTrace {
                agent_id: "dag-plan-validation".into(),
                description: "dag plan validation".into(),
                output: format!("dropped {} out-of-range dependencies", dropped.len()),
                start_time: Utc::now(),
                duration_ms: 0,
                events,
            },
        );
    }
    dag.topological_waves()
}

/// Mutable per-node execution state shared across dag waves.
struct DagRun {
    tasks: Vec<String>,
    /// Node id → dependency node ids (only valid edges).
    deps: Vec<Vec<usize>>,
    /// Node id → worker output for completed tasks.
    outputs: Vec<Option<String>>,
    /// Node id → the task failed or was skipped.
    failed: Vec<bool>,
}

impl DagRun {
    fn new(tasks: Vec<String>, edges: &[(usize, usize)]) -> Self {
        let mut deps: Vec<Vec<usize>> = vec![Vec::new(); tasks.len()];
        for &(task, dependency) in edges {
            if task < tasks.len() && dependency < tasks.len() {
                deps[task].push(dependency);
            }
        }
        let node_count = tasks.len();
        Self {
            tasks,
            deps,
            outputs: vec![None; node_count],
            failed: vec![false; node_count],
        }
    }
}

/// Execute one wave: skip tasks whose dependency failed (counting toward the
/// circuit breaker), run the rest concurrently (bounded by the semaphore).
async fn run_wave(
    ctx: &Context,
    env: &WorkerEnv,
    state: &Arc<SwarmState>,
    run: &mut DagRun,
    wave_number: usize,
    wave: &[usize],
) -> RoundOutcome {
    let mut outcome = RoundOutcome::default();
    let mut handles = Vec::new();
    for &node in wave {
        if run.deps[node].iter().any(|&dep| run.failed[dep]) {
            run.failed[node] = true;
            outcome.errors.push(skip_failed_dependency(
                ctx,
                state,
                wave_number,
                node,
                &run.tasks[node],
            ));
            continue;
        }
        let prompt = build_dag_task_prompt(&run.tasks[node], &run.deps[node], &run.tasks, &run.outputs);
        let worker_env = env.clone();
        let bare_task = run.tasks[node].clone();
        handles.push(tokio::spawn(async move {
            (node, run_worker(worker_env, wave_number, node, prompt, Some(bare_task)).await)
        }));
    }

    // On abort the join is dropped, detaching in-flight worker spawns (see
    // module docs).
    let results = tokio::select! {
        () = ctx.abort.cancelled() => return RoundOutcome { aborted: true, ..RoundOutcome::default() },
        results = join_all(handles) => results,
    };
    collect_wave_results(results, &mut run.outputs, &mut run.failed, &mut outcome);
    outcome
}

/// Skip a task whose dependency failed: trace an Error event, count the
/// failure toward the circuit breaker, and return the error message.
fn skip_failed_dependency(
    ctx: &Context,
    state: &SwarmState,
    wave: usize,
    node: usize,
    task_text: &str,
) -> String {
    let worker_id = format!("worker-{wave}-{node}");
    let message = format!("{worker_id}: skipped: dependency failed");
    state.record_failure(ctx.config.circuit_breaker);
    state.finish_trace(
        &ctx.trace_tx,
        AgentTrace {
            agent_id: worker_id,
            description: task_text.to_string(),
            output: message.clone(),
            start_time: Utc::now(),
            duration_ms: 0,
            events: vec![
                TraceEvent::Error {
                    error: "skipped: dependency failed".into(),
                },
                TraceEvent::Termination {
                    reason: TerminationReason::Error(message.clone()),
                },
            ],
        },
    );
    message
}

/// One spawned worker's join result, tagged with its node id.
type WaveResult = Result<(usize, Result<(String, String), String>), JoinError>;

/// Fold one wave's join results into per-node outputs/failures and the outcome.
fn collect_wave_results(
    results: Vec<WaveResult>,
    outputs: &mut [Option<String>],
    failed: &mut [bool],
    outcome: &mut RoundOutcome,
) {
    for result in results {
        match result {
            Ok((node, Ok((worker_id, output)))) => {
                outputs[node] = Some(output.clone());
                outcome.successes.push((worker_id, output));
            }
            Ok((node, Err(error))) => {
                failed[node] = true;
                outcome.errors.push(error);
            }
            Err(join_error) => {
                outcome.errors.push(format!("worker task failed to join: {join_error}"));
            }
        }
    }
}

/// Worker prompt for a dag task: the task text plus a `Context from previous
/// steps` section listing each completed dependency's output (truncated to
/// 1000 chars). Omitted entirely when no dependency produced output.
fn build_dag_task_prompt(
    task_text: &str,
    deps: &[usize],
    tasks: &[String],
    outputs: &[Option<String>],
) -> String {
    let context: Vec<String> = deps
        .iter()
        .filter_map(|&dep| {
            outputs[dep]
                .as_ref()
                .map(|output| format!("- {}: {}", tasks[dep], truncate(output, DEP_OUTPUT_CHARS)))
        })
        .collect();
    if context.is_empty() {
        task_text.to_string()
    } else {
        format!(
            "{task_text}\n\nContext from previous steps:\n{}",
            context.join("\n")
        )
    }
}
