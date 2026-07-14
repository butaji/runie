//! Black-box tests for runie-patterns Phase 2: SwarmPattern (parallel + delegation).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use runie_patterns::{
    AgentTrace, Context, Pattern, PatternConfig, PatternOutput, PatternRegistry, SwarmPattern,
    SwarmVariant, TerminationReason, TraceEvent, WorkerRunner, WorkerTask,
};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

fn ok(text: &str) -> Result<String, String> {
    Ok(text.to_string())
}

fn err(msg: &str) -> Result<String, String> {
    Err(msg.to_string())
}

/// Mock runner: FIFO queue of canned responses, call recording, optional
/// sleep, optional abort-token cancellation on the first worker call.
#[derive(Default)]
struct MockRunner {
    calls: Arc<Mutex<Vec<WorkerTask>>>,
    responses: Arc<Mutex<VecDeque<Result<String, String>>>>,
    sleep: Option<Duration>,
    abort_on_worker: Option<CancellationToken>,
}

impl MockRunner {
    fn new(responses: Vec<Result<String, String>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into())),
            ..Self::default()
        }
    }

    fn with_sleep(mut self, d: Duration) -> Self {
        self.sleep = Some(d);
        self
    }

    fn with_abort_on_worker(mut self, token: CancellationToken) -> Self {
        self.abort_on_worker = Some(token);
        self
    }

    fn calls_with_prefix(&self, prefix: &str) -> Vec<WorkerTask> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.id.starts_with(prefix))
            .cloned()
            .collect()
    }
}

#[async_trait::async_trait]
impl WorkerRunner for MockRunner {
    async fn run(&self, task: WorkerTask) -> Result<String> {
        if task.id.starts_with("worker-") {
            if let Some(token) = &self.abort_on_worker {
                token.cancel();
            }
        }
        self.calls.lock().unwrap().push(task);
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err("no canned response".into()));
        if let Some(d) = self.sleep {
            tokio::time::sleep(d).await;
        }
        response.map_err(anyhow::Error::msg)
    }
}

fn test_config() -> PatternConfig {
    PatternConfig {
        active: "swarm".into(),
        workers: 3,
        max_rounds: 5,
        timeout_ms: 5_000,
        max_retries: 2,
        circuit_breaker: 3,
    }
}

fn make_ctx(
    config: PatternConfig,
    runner: Arc<dyn WorkerRunner>,
    abort: CancellationToken,
) -> (Context, mpsc::UnboundedReceiver<AgentTrace>) {
    let (trace_tx, trace_rx) = mpsc::unbounded_channel();
    let workers = config.workers;
    let ctx = Context {
        config,
        models: vec![("mock".into(), "echo".into())],
        semaphore: Arc::new(Semaphore::new(workers)),
        trace_tx,
        abort,
        runner,
    };
    (ctx, trace_rx)
}

fn trace_ids(out: &PatternOutput) -> Vec<String> {
    out.traces.iter().map(|t| t.agent_id.clone()).collect()
}

#[tokio::test]
async fn swarm_parallel_happy_path() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![
        ok(r#"["task one","task two","task three"]"#),
        ok("out-a"),
        ok("out-b"),
        ok("out-c"),
        ok("final"),
    ]));
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = SwarmPattern::parallel().execute(&ctx, "do things").await?;

    assert_eq!(out.result, "final");
    assert_eq!(out.termination, TerminationReason::Completed);

    let ids = trace_ids(&out);
    assert_eq!(ids.len(), 5);
    for expected in [
        "leader-plan-1",
        "worker-1-0",
        "worker-1-1",
        "worker-1-2",
        "leader-synthesize",
    ] {
        assert!(ids.contains(&expected.to_string()), "missing trace {expected} in {ids:?}");
    }

    // Workers received the planned task texts; the plan prompt carries the input.
    let workers = runner.calls_with_prefix("worker-");
    let mut worker_prompts: Vec<String> = workers.iter().map(|t| t.prompt.clone()).collect();
    worker_prompts.sort();
    assert_eq!(worker_prompts, vec!["task one", "task three", "task two"]);
    assert!(workers.iter().all(|t| !t.read_only));

    let plan_calls = runner.calls_with_prefix("leader-plan-");
    assert_eq!(plan_calls.len(), 1);
    assert!(plan_calls[0].prompt.contains("[swarm-plan parallel]"));
    assert!(plan_calls[0].prompt.contains("do things"));
    assert!(plan_calls[0].read_only);
    Ok(())
}

#[tokio::test]
async fn swarm_plan_parse_failure_falls_back_to_single_task() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![
        ok("sorry, no JSON here"),
        ok("worker out"),
        ok("final"),
    ]));
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = SwarmPattern::parallel().execute(&ctx, "the original task").await?;

    assert_eq!(out.result, "final");
    assert_eq!(out.termination, TerminationReason::Completed);
    let workers = runner.calls_with_prefix("worker-");
    assert_eq!(workers.len(), 1, "fallback dispatches exactly one task");
    assert_eq!(workers[0].prompt, "the original task");
    Ok(())
}

#[tokio::test]
async fn swarm_plan_accepts_object_array_with_task_field() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![
        ok(r#"[{"task": "object task"}, {"prompt": "prompt task"}]"#),
        ok("o1"),
        ok("o2"),
        ok("final"),
    ]));
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = SwarmPattern::parallel().execute(&ctx, "objects").await?;

    assert_eq!(out.termination, TerminationReason::Completed);
    let mut prompts: Vec<String> = runner
        .calls_with_prefix("worker-")
        .iter()
        .map(|t| t.prompt.clone())
        .collect();
    prompts.sort();
    assert_eq!(prompts, vec!["object task", "prompt task"]);
    Ok(())
}

#[tokio::test]
async fn swarm_worker_retry_on_failure() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![
        ok(r#"["flaky task"]"#),
        err("flaky"),
        ok("recovered"),
        ok("final"),
    ]));
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = SwarmPattern::parallel().execute(&ctx, "retry me").await?;

    assert_eq!(out.result, "final");
    assert_eq!(out.termination, TerminationReason::Completed);
    assert_eq!(runner.calls_with_prefix("worker-").len(), 2, "one retry");
    let worker_trace = out
        .traces
        .iter()
        .find(|t| t.agent_id == "worker-1-0")
        .expect("worker trace");
    assert!(
        worker_trace
            .events
            .iter()
            .any(|e| matches!(e, TraceEvent::Error { error } if error.contains("flaky"))),
        "worker trace should record the failed attempt"
    );
    Ok(())
}

#[tokio::test]
async fn swarm_circuit_breaker_stops_dispatch() -> Result<()> {
    let mut responses = vec![ok(r#"["t1","t2","t3","t4","t5"]"#)];
    responses.extend((0..12).map(|_| err("boom")));
    let config = PatternConfig {
        workers: 2,
        circuit_breaker: 3,
        ..test_config()
    };
    let runner = Arc::new(MockRunner::new(responses));
    let (ctx, _rx) = make_ctx(config, runner.clone(), CancellationToken::new());

    let out = SwarmPattern::parallel().execute(&ctx, "doomed").await?;

    match &out.termination {
        TerminationReason::Error(msg) => assert!(msg.contains("circuit breaker"), "got {msg}"),
        other => panic!("expected circuit breaker error, got {other:?}"),
    }
    assert!(
        runner.calls_with_prefix("leader-synthesize").is_empty(),
        "synthesis must be skipped when the circuit breaker trips"
    );
    let attempts = runner.calls_with_prefix("worker-").len();
    assert!(attempts < 15, "dispatch should stop early, got {attempts} attempts");
    Ok(())
}

#[tokio::test]
async fn swarm_worker_timeout_counts_as_failure() -> Result<()> {
    let config = PatternConfig {
        timeout_ms: 50,
        max_retries: 1,
        ..test_config()
    };
    let runner = Arc::new(
        MockRunner::new(vec![ok(r#"["slow task"]"#), ok("late"), ok("late")])
            .with_sleep(Duration::from_millis(300)),
    );
    let (ctx, _rx) = make_ctx(config, runner.clone(), CancellationToken::new());
    let start = Instant::now();

    let out = SwarmPattern::parallel().execute(&ctx, "slow").await?;

    assert!(start.elapsed() < Duration::from_secs(2), "test must stay fast");
    match &out.termination {
        TerminationReason::Error(msg) => assert!(msg.contains("timed out"), "got {msg}"),
        other => panic!("expected error termination, got {other:?}"),
    }
    assert_eq!(runner.calls_with_prefix("worker-").len(), 2, "1 attempt + 1 retry");
    let worker_trace = out
        .traces
        .iter()
        .find(|t| t.agent_id == "worker-1-0")
        .expect("worker trace");
    assert!(
        worker_trace
            .events
            .iter()
            .any(|e| matches!(e, TraceEvent::Error { error } if error.contains("timed out"))),
        "worker trace should record the timeout"
    );
    Ok(())
}

#[tokio::test]
async fn swarm_abort_mid_execute() -> Result<()> {
    let abort = CancellationToken::new();
    let runner = Arc::new(
        MockRunner::new(vec![ok(r#"["t1","t2"]"#), ok("never"), ok("never")])
            .with_sleep(Duration::from_millis(500))
            .with_abort_on_worker(abort.clone()),
    );
    let (ctx, _rx) = make_ctx(test_config(), runner, abort);
    let start = Instant::now();

    let out = SwarmPattern::parallel().execute(&ctx, "abort me").await?;

    assert_eq!(out.termination, TerminationReason::Error("aborted".into()));
    assert!(start.elapsed() < Duration::from_secs(2), "abort must be fast");
    Ok(())
}

#[tokio::test]
async fn swarm_delegation_early_finish_on_empty_plan() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![
        ok(r#"["t1","t2"]"#),
        ok("r1"),
        ok("r2"),
        ok("[]"),
        ok("final"),
    ]));
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = SwarmPattern::delegation().execute(&ctx, "delegate this").await?;

    assert_eq!(out.result, "final");
    assert_eq!(out.termination, TerminationReason::Completed);
    let ids = trace_ids(&out);
    assert!(ids.contains(&"leader-plan-1".to_string()));
    assert!(ids.contains(&"leader-plan-2".to_string()));

    let synth = runner.calls_with_prefix("leader-synthesize");
    assert_eq!(synth.len(), 1, "synthesis called once");
    assert!(synth[0].prompt.contains("r1") && synth[0].prompt.contains("r2"));

    let plan2 = runner.calls_with_prefix("leader-plan-2");
    assert_eq!(plan2.len(), 1);
    assert!(
        plan2[0].prompt.contains("Previous rounds:"),
        "plan round 2 includes the prior-rounds summary"
    );
    Ok(())
}

#[tokio::test]
async fn swarm_delegation_max_rounds() -> Result<()> {
    let config = PatternConfig {
        max_rounds: 2,
        ..test_config()
    };
    let runner = Arc::new(MockRunner::new(vec![
        ok(r#"["t"]"#),
        ok("r1"),
        ok(r#"["t"]"#),
        ok("r2"),
        ok("final"),
    ]));
    let (ctx, _rx) = make_ctx(config, runner.clone(), CancellationToken::new());

    let out = SwarmPattern::delegation().execute(&ctx, "loop").await?;

    assert_eq!(out.termination, TerminationReason::MaxRoundsReached);
    assert_eq!(out.result, "final");
    assert_eq!(
        runner.calls_with_prefix("leader-plan-").len(),
        2,
        "exactly 2 plan rounds"
    );
    assert_eq!(runner.calls_with_prefix("leader-synthesize").len(), 1);
    Ok(())
}

#[test]
fn swarm_registry_and_metadata() {
    let registry = PatternRegistry::default();
    assert!(registry.names().contains(&"single"));
    assert!(registry.names().contains(&"swarm"));

    let parallel = SwarmPattern::parallel();
    let delegation = SwarmPattern::delegation();
    assert_eq!(parallel.name(), "swarm");
    assert_eq!(delegation.name(), "swarm");
    assert_eq!(parallel.variant(), &SwarmVariant::Parallel);
    assert_eq!(delegation.variant(), &SwarmVariant::Delegation);

    let desc = parallel.description().to_lowercase();
    assert!(
        desc.contains("coordinated") || desc.contains("multi-agent"),
        "description should mention coordinated/multi-agent: {desc}"
    );
}

#[tokio::test]
async fn swarm_workers_fall_back_to_leader_model() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![
        ok(r#"["t1","t2"]"#),
        ok("o1"),
        ok("o2"),
        ok("final"),
    ]));
    // make_ctx configures a single ("mock", "echo") model.
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = SwarmPattern::parallel().execute(&ctx, "models").await?;

    assert_eq!(out.termination, TerminationReason::Completed);
    let workers = runner.calls_with_prefix("worker-");
    assert_eq!(workers.len(), 2);
    for task in workers {
        assert_eq!(
            (task.provider.as_str(), task.model.as_str()),
            ("mock", "echo"),
            "worker {} must reuse the leader model",
            task.id
        );
    }
    Ok(())
}
