//! Pattern execution wiring for the TUI (PATTERNS.md Phase 2).
//!
//! Bridges `runie_patterns` into the UiActor turn flow:
//! - [`TuiWorkerRunner`] implements [`WorkerRunner`] over
//!   `runie_agent::subagent::run_subagent`, building providers through the
//!   `ProviderActor` handle.
//! - [`should_use_pattern`] decides whether a mode intercepts the agent turn.
//! - [`swarm_for_variant`] picks the swarm engine variant.
//! - [`publish_worker_rows`] turns worker traces into feed rows.

#![allow(clippy::too_many_lines)]

use runie_core::actors::RactorProviderHandle;
use runie_core::bus::EventBus;
use runie_core::model::ThinkingLevel;
use runie_core::Event;
use runie_patterns::{
    AgentTrace, PatternOutput, SwarmPattern, TerminationReason, TraceEvent, WorkerRunner, WorkerTask,
};

/// Worker runner backed by the `ProviderActor`: each `run()` builds the
/// provider for the task's provider/model and executes one scoped subagent
/// turn (no skills context, no system prompt), returning the final assistant
/// text. Cheap to clone — the handle is an `ActorRef` internally.
#[derive(Clone)]
pub struct TuiWorkerRunner {
    provider_handle: RactorProviderHandle,
    thinking: ThinkingLevel,
    max_iterations: usize,
}

impl TuiWorkerRunner {
    /// Create a runner that builds providers via `provider_handle` and runs
    /// subagents with the given thinking level and iteration cap.
    pub fn new(provider_handle: RactorProviderHandle, thinking: ThinkingLevel, max_iterations: usize) -> Self {
        Self { provider_handle, thinking, max_iterations }
    }
}

/// Resolve the effective (provider, model) for a worker task.
///
/// The mock override mirrors `AgentActor::build_provider_turn`: when the mock
/// provider is enabled (`RUNIE_MOCK` / `--mock`), every agent — leader and
/// workers alike — runs on mock/echo regardless of the configured pair.
pub(crate) fn resolve_provider_model(task: &WorkerTask, mock_enabled: bool) -> (String, String) {
    if mock_enabled {
        ("mock".to_owned(), "echo".to_owned())
    } else {
        (task.provider.clone(), task.model.clone())
    }
}

/// Whether `mode.active` selects a pattern-driven turn that intercepts the
/// single-agent turn.
pub(crate) fn should_use_pattern(active: &str) -> bool {
    pattern_for_mode(active, None).is_some()
}

/// Pattern engine for the active mode and session swarm variant.
pub(crate) fn pattern_for_mode(active: &str, variant: Option<&str>) -> Option<Box<dyn runie_patterns::Pattern>> {
    match active {
        "swarm" => Some(Box::new(swarm_for_variant(variant))),
        "improve" => Some(Box::new(runie_patterns::ImprovePattern)),
        _ => None,
    }
}

/// Swarm engine for the session variant.
pub(crate) fn swarm_for_variant(variant: Option<&str>) -> SwarmPattern {
    match variant {
        Some("delegation") => SwarmPattern::delegation(),
        Some("dag") => SwarmPattern::dag(),
        _ => SwarmPattern::parallel(),
    }
}

/// Minimum time a worker row stays in the Running state before the Finished
/// update is published. Without this, Spawned and Finished events are batched
/// in a single UiActor iteration and the running row is never rendered.
const WORKER_RUNNING_VISIBILITY_MS: u64 = 200;

/// Whether a trace id should surface as a feed row.
///
/// Swarm workers use `worker-*`; improve uses `improve-generate-*`,
/// `improve-review-*`, and `improve-revise-*`. Internal leader/plan/synthesis traces
/// are filtered out.
fn is_visible_worker_trace(agent_id: &str) -> bool {
    agent_id.starts_with("worker-")
        || agent_id.starts_with("improve-generate-")
        || agent_id.starts_with("improve-review-")
        || agent_id.starts_with("improve-revise-")
}

/// Publish one feed row per worker trace (`worker-*` and `improve-*` ids; leader
/// plan/synthesis traces are internal to the pattern).
///
/// The patterns emit traces only on worker completion, so rows are published
/// post-hoc after `execute()` returns: each worker gets a Spawned row, a short
/// visibility delay, then its Finished update. Live in-flight rows would need
/// per-trace streaming, which the pattern crate does not offer yet.
pub(crate) async fn publish_worker_rows(bus: &EventBus<Event>, traces: &[AgentTrace], model: &str) {
    for trace in traces {
        if !is_visible_worker_trace(&trace.agent_id) {
            continue;
        }
        let id = trace.agent_id.clone();
        bus.publish(Event::PatternWorkerSpawned {
            id: id.clone(),
            description: truncate_description(&trace.description),
            model: model.to_owned(),
        });
        tokio::time::sleep(std::time::Duration::from_millis(
            WORKER_RUNNING_VISIBILITY_MS,
        ))
        .await;
        let (status, output) = worker_outcome(trace);
        bus.publish(Event::PatternWorkerFinished { id, status, duration_ms: trace.duration_ms, output });
    }
}

/// Trim a task description to one short line for the feed row.
fn truncate_description(description: &str) -> String {
    const MAX: usize = 60;
    let one_line = description.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= MAX {
        return one_line;
    }
    let mut out: String = one_line.chars().take(MAX - 1).collect();
    out.push('…');
    out
}

/// Map a worker trace to its feed-row status and output.
///
/// `"completed"` is the only status the projection treats as success. The
/// trace carries the worker's output text (or failure message), which the
/// expanded feed row renders as the transcript body.
fn worker_outcome(trace: &AgentTrace) -> (String, String) {
    let failed = trace.events.iter().any(|e| {
        matches!(
            e,
            TraceEvent::Termination { reason: TerminationReason::Error(_) | TerminationReason::Timeout }
        )
    });
    if failed {
        let output = if trace.output.is_empty() {
            trace
                .events
                .iter()
                .filter_map(|e| match e {
                    TraceEvent::Error { error } => Some(error.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            trace.output.clone()
        };
        ("failed".to_owned(), output)
    } else {
        ("completed".to_owned(), trace.output.clone())
    }
}

/// Publish the terminal events for a finished pattern run.
///
/// The pattern replaces the agent turn, so this mirrors the agent actor's
/// contract exactly — worker feed rows, then the final assistant message,
/// then `TurnComplete` + `Done` on success or `Error` + `Done` on failure
/// (same shape as `AgentActor::publish_error_and_done`). The TurnActor
/// bridges `Done`/`Error` into `TurnCompleted`/`TurnErrored`, which releases
/// the UiActor turn guard. Callers must skip this entirely on abort: the
/// `Event::Abort` path already finalized the turn.
pub(crate) async fn publish_pattern_outcome(
    bus: &EventBus<Event>,
    id: &str,
    outcome: anyhow::Result<PatternOutput>,
    model: &str,
    start: std::time::Instant,
    circuit_breaker_threshold: u32,
) {
    match outcome {
        Ok(output) => {
            publish_worker_rows(bus, &output.traces, model).await;
            if output.circuit_breaker_tripped {
                bus.publish(Event::CircuitBreakerTripped {
                    failures: circuit_breaker_threshold,
                    threshold: circuit_breaker_threshold,
                });
            }
            match output.termination {
                TerminationReason::Completed | TerminationReason::MaxRoundsReached | TerminationReason::Approved => {
                    if !output.result.is_empty() {
                        bus.publish(Event::Response {
                            id: id.to_owned(),
                            content: output.result,
                            role: String::new(),
                            timestamp: runie_core::message::now(),
                            provider: String::new(),
                        });
                    }
                    bus.publish(Event::TurnComplete {
                        id: id.to_owned(),
                        duration_secs: start.elapsed().as_secs_f64(),
                    });
                    bus.publish(Event::Done { id: id.to_owned() });
                }
                TerminationReason::Error(message) => {
                    publish_error_and_done(bus, id, message);
                }
                TerminationReason::Timeout => {
                    publish_error_and_done(bus, id, "pattern timed out".to_owned());
                }
            }
        }
        Err(error) => {
            publish_error_and_done(bus, id, format!("Pattern error: {error}"));
        }
    }
}

/// `Error` + `Done` — the same terminal pair the agent actor publishes.
fn publish_error_and_done(bus: &EventBus<Event>, id: &str, message: String) {
    bus.publish(Event::Error { id: id.to_owned(), message });
    bus.publish(Event::Done { id: id.to_owned() });
}

#[async_trait::async_trait]
impl WorkerRunner for TuiWorkerRunner {
    async fn run(&self, task: WorkerTask) -> anyhow::Result<String> {
        let (provider_key, model) = resolve_provider_model(&task, runie_core::provider::is_mock_enabled());
        let built = self
            .provider_handle
            .build(provider_key.clone(), model.clone())
            .await
            .map_err(|e| anyhow::anyhow!("provider build failed for {provider_key}/{model}: {e}"))?;
        let text = runie_agent::subagent::run_subagent(
            &task.prompt,
            &provider_key,
            &model,
            &built,
            self.thinking,
            task.read_only,
            "", // workers are scoped: no skills context
            "", // workers are scoped: no system prompt
            self.max_iterations,
        )
        .await?;
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_patterns::SwarmVariant;

    fn task(provider: &str, model: &str) -> WorkerTask {
        WorkerTask {
            id: "worker-1-0".into(),
            prompt: "do something".into(),
            provider: provider.into(),
            model: model.into(),
            read_only: false,
        }
    }

    #[test]
    fn mock_override_forces_mock_echo() {
        let (provider, model) = resolve_provider_model(&task("openai", "gpt-4o"), true);
        assert_eq!((provider.as_str(), model.as_str()), ("mock", "echo"));
    }

    #[test]
    fn without_mock_uses_task_pair() {
        let (provider, model) = resolve_provider_model(&task("openai", "gpt-4o"), false);
        assert_eq!((provider.as_str(), model.as_str()), ("openai", "gpt-4o"));
    }

    #[test]
    fn pattern_intercepts_pattern_modes() {
        assert!(!should_use_pattern("single"));
        assert!(should_use_pattern("swarm"));
        assert!(should_use_pattern("improve"));
        assert!(!should_use_pattern("unknown"));
        assert!(!should_use_pattern(""));
    }

    #[test]
    fn mode_pattern_mapping() {
        assert!(pattern_for_mode("swarm", None).is_some());
        assert!(pattern_for_mode("improve", None).is_some());
        assert!(pattern_for_mode("single", None).is_none());
    }

    #[test]
    fn swarm_variant_mapping() {
        assert!(matches!(
            swarm_for_variant(Some("delegation")).variant(),
            SwarmVariant::Delegation
        ));
        assert!(matches!(
            swarm_for_variant(Some("parallel")).variant(),
            SwarmVariant::Parallel
        ));
        assert!(matches!(
            swarm_for_variant(Some("dag")).variant(),
            SwarmVariant::Dag
        ));
        // No variant configured defaults to parallel.
        assert!(matches!(
            swarm_for_variant(None).variant(),
            SwarmVariant::Parallel
        ));
    }

    fn trace(id: &str, failed: bool) -> AgentTrace {
        let mut events = vec![TraceEvent::Handoff { from: "leader".into(), to: id.into() }];
        if failed {
            events.push(TraceEvent::Error { error: "boom".into() });
            events.push(TraceEvent::Termination { reason: TerminationReason::Error("boom".into()) });
        } else {
            events.push(TraceEvent::Termination { reason: TerminationReason::Completed });
        }
        AgentTrace {
            agent_id: id.into(),
            description: format!("task for {id}"),
            output: if failed { "boom".into() } else { "ok".into() },
            start_time: chrono::Utc::now(),
            duration_ms: 42,
            events,
        }
    }

    /// Worker traces produce Spawned + Finished rows; leader traces are
    /// skipped; failed workers map to a non-"completed" status carrying the
    /// error text.
    #[tokio::test]
    async fn worker_rows_published_for_worker_traces_only() {
        let bus = EventBus::<Event>::new(16);
        let mut rx = bus.subscribe();
        let traces = vec![
            trace("leader-plan-1", false),
            trace("worker-1-0", false),
            trace("worker-1-1", true),
            trace("leader-synthesize", false),
        ];
        publish_worker_rows(&bus, &traces, "echo").await;

        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }
        assert_eq!(
            events.len(),
            4,
            "one Spawned + one Finished per worker trace: {events:?}"
        );
        assert!(matches!(
            &events[0],
            Event::PatternWorkerSpawned { id, description, model }
                if id == "worker-1-0" && description == "task for worker-1-0" && model == "echo"
        ));
        assert!(matches!(
            &events[1],
            Event::PatternWorkerFinished { id, status, duration_ms, output }
                if id == "worker-1-0" && status == "completed" && *duration_ms == 42 && output == "ok"
        ));
        assert!(matches!(
            &events[2],
            Event::PatternWorkerSpawned { id, .. } if id == "worker-1-1"
        ));
        assert!(matches!(
            &events[3],
            Event::PatternWorkerFinished { id, status, output, .. }
                if id == "worker-1-1" && status != "completed" && output.contains("boom")
        ));
    }

    /// Improve traces (`improve-generate-*`, `improve-review-*`,
    /// `improve-revise-*`) produce feed rows the same way swarm workers do.
    #[tokio::test]
    async fn worker_rows_published_for_improve_traces() {
        let bus = EventBus::<Event>::new(16);
        let mut rx = bus.subscribe();
        let traces = vec![
            trace("improve-generate-1", false),
            trace("improve-review-1", false),
            trace("improve-revise-1", false),
            trace("leader-synthesize", false),
        ];
        publish_worker_rows(&bus, &traces, "echo").await;

        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }
        let spawned: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::PatternWorkerSpawned { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            spawned,
            vec!["improve-generate-1".to_string(), "improve-review-1".to_string(), "improve-revise-1".to_string(),],
            "improve traces must produce rows; leader trace must be hidden: {events:?}"
        );
    }

    fn output(result: &str, termination: TerminationReason) -> PatternOutput {
        PatternOutput { result: result.into(), termination, traces: Vec::new(), circuit_breaker_tripped: false }
    }

    fn drain(rx: &mut runie_core::bus::Receiver<Event>) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }
        events
    }

    /// Success: final message, then TurnComplete, then Done (agent contract).
    #[tokio::test]
    async fn outcome_success_publishes_response_turn_complete_done() {
        let bus = EventBus::<Event>::new(16);
        let mut rx = bus.subscribe();
        publish_pattern_outcome(
            &bus,
            "req.0",
            Ok(output("final answer", TerminationReason::Completed)),
            "echo",
            std::time::Instant::now(),
            3,
        )
        .await;
        let events = drain(&mut rx);
        assert_eq!(events.len(), 3, "{events:?}");
        assert!(matches!(
            &events[0],
            Event::Response { id, content, .. } if id == "req.0" && content == "final answer"
        ));
        assert!(matches!(&events[1], Event::TurnComplete { id, .. } if id == "req.0"));
        assert!(matches!(&events[2], Event::Done { id } if id == "req.0"));
    }

    /// An empty result still finalizes the turn, just without a Response.
    #[tokio::test]
    async fn outcome_success_with_empty_result_skips_response() {
        let bus = EventBus::<Event>::new(16);
        let mut rx = bus.subscribe();
        publish_pattern_outcome(
            &bus,
            "req.0",
            Ok(output("", TerminationReason::MaxRoundsReached)),
            "echo",
            std::time::Instant::now(),
            3,
        )
        .await;
        let events = drain(&mut rx);
        assert_eq!(events.len(), 2, "{events:?}");
        assert!(matches!(&events[0], Event::TurnComplete { .. }));
        assert!(matches!(&events[1], Event::Done { .. }));
    }

    /// Failed termination: Error + Done (AgentActor::publish_error_and_done
    /// shape), no Response.
    #[tokio::test]
    async fn outcome_error_termination_publishes_error_and_done() {
        let bus = EventBus::<Event>::new(16);
        let mut rx = bus.subscribe();
        publish_pattern_outcome(
            &bus,
            "req.0",
            Ok(output(
                "",
                TerminationReason::Error("all workers failed".into()),
            )),
            "echo",
            std::time::Instant::now(),
            3,
        )
        .await;
        let events = drain(&mut rx);
        assert_eq!(events.len(), 2, "{events:?}");
        assert!(matches!(
            &events[0],
            Event::Error { id, message } if id == "req.0" && message == "all workers failed"
        ));
        assert!(matches!(&events[1], Event::Done { id } if id == "req.0"));
    }

    /// Execute error: same Error + Done shape, message carries the source.
    #[tokio::test]
    async fn outcome_execute_error_publishes_error_and_done() {
        let bus = EventBus::<Event>::new(16);
        let mut rx = bus.subscribe();
        publish_pattern_outcome(
            &bus,
            "req.0",
            Err(anyhow::anyhow!("kaboom")),
            "echo",
            std::time::Instant::now(),
            3,
        )
        .await;
        let events = drain(&mut rx);
        assert_eq!(events.len(), 2, "{events:?}");
        assert!(matches!(
            &events[0],
            Event::Error { id, message } if id == "req.0" && message.contains("kaboom")
        ));
        assert!(matches!(&events[1], Event::Done { id } if id == "req.0"));
    }
}
