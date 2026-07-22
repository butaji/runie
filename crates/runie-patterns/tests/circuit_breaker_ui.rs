//! Tests for circuit breaker UI feedback behavior.
//!
//! Verifies that PatternOutput.circuit_breaker_tripped is set correctly
//! when the swarm circuit breaker trips, and tests the swarm pattern's
//! interaction with the circuit breaker state machine.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use runie_patterns::{
    AgentTrace, Context, Pattern, PatternConfig, SwarmPattern, TerminationReason,
    WorkerRunner, WorkerTask,
};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

fn ok(text: &str) -> Result<String, String> {
    Ok(text.to_string())
}

fn err(msg: &str) -> Result<String, String> {
    Err(msg.to_string())
}

/// Mock runner: FIFO queue of canned responses.
#[derive(Default)]
struct MockRunner {
    responses: Arc<Mutex<VecDeque<Result<String, String>>>>,
}

impl MockRunner {
    fn new(responses: Vec<Result<String, String>>) -> Self {
        Self { responses: Arc::new(Mutex::new(responses.into())) }
    }
}

#[async_trait::async_trait]
impl WorkerRunner for MockRunner {
    async fn run(&self, _task: WorkerTask) -> Result<String> {
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err("no canned response".into()));
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
        doom_loop_threshold: 5,
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

/// Verifies that circuit_breaker_tripped is false when the circuit never trips.
#[tokio::test]
async fn circuit_breaker_not_tripped_on_success() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![
        ok(r#"["t1","t2"]"#),
        ok("out-a"),
        ok("out-b"),
        ok("final"),
    ]));
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = SwarmPattern::parallel().execute(&ctx, "succeed").await?;

    assert_eq!(out.termination, TerminationReason::Completed);
    assert!(!out.circuit_breaker_tripped, "circuit breaker should not trip on success");
    Ok(())
}

/// Verifies that circuit_breaker_tripped is true when the circuit trips.
#[tokio::test]
async fn circuit_breaker_tripped_on_failure() -> Result<()> {
    let mut responses = vec![ok(r#"["t1","t2","t3","t4","t5"]"#)];
    responses.extend((0..12).map(|_| err("boom")));
    let config = PatternConfig { workers: 2, circuit_breaker: 3, ..test_config() };
    let runner = Arc::new(MockRunner::new(responses));
    let (ctx, _rx) = make_ctx(config, runner.clone(), CancellationToken::new());

    let out = SwarmPattern::parallel().execute(&ctx, "doomed").await?;

    match &out.termination {
        TerminationReason::Error(msg) => assert!(msg.contains("circuit breaker"), "got {msg}"),
        other => panic!("expected circuit breaker error, got {other:?}"),
    }
    assert!(out.circuit_breaker_tripped, "circuit breaker should trip after consecutive failures");
    Ok(())
}

/// Verifies circuit_breaker_tripped is false when breaker trips but some tasks succeed first.
#[tokio::test]
async fn circuit_breaker_tripped_after_partial_success() -> Result<()> {
    // With workers=2, 4 tasks in 2 waves:
    //   Wave 1: t1 (ok), t2 (fail) → 1 success resets, 1 failure = 1 failure
    //   Wave 2: t3 (fail), t4 (fail) → 2 more failures = 3 total → BREAKER TRIPS
    // Responses (FIFO): plan + 4 tasks = 5 total
    // Breaker trips after execute_round completes, synthesis is never called
    let responses = vec![
        ok(r#"["t1","t2","t3","t4"]"#), // plan: 4 tasks
        ok("out-a"),                     // t1: success (resets failure count to 0)
        err("boom"),                     // t2: failure 1
        err("boom"),                     // t3: failure 2
        err("boom"),                     // t4: failure 3 → BREAKER TRIPS
    ];
    let config = PatternConfig { workers: 2, circuit_breaker: 3, max_retries: 0, ..test_config() }; // max_retries=0 = 1 attempt
    let runner = Arc::new(MockRunner::new(responses));
    let (ctx, _rx) = make_ctx(config, runner.clone(), CancellationToken::new());

    let out = SwarmPattern::parallel().execute(&ctx, "partial failure").await?;

    assert!(out.circuit_breaker_tripped, "circuit breaker should trip even with partial success");
    Ok(())
}

/// Verifies circuit_breaker_tripped is false on delegation max rounds.
#[tokio::test]
async fn circuit_breaker_not_tripped_on_max_rounds() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![
        ok(r#"["t"]"#),
        ok("r1"),
        ok(r#"["t"]"#),
        ok("r2"),
        ok("final"),
    ]));
    let config = PatternConfig { max_rounds: 2, ..test_config() };
    let (ctx, _rx) = make_ctx(config, runner.clone(), CancellationToken::new());

    let out = SwarmPattern::delegation().execute(&ctx, "max rounds").await?;

    assert_eq!(out.termination, TerminationReason::MaxRoundsReached);
    assert!(!out.circuit_breaker_tripped, "circuit breaker should not trip on max rounds");
    Ok(())
}

/// Verifies circuit_breaker_tripped is set correctly for DAG variant.
#[tokio::test]
async fn circuit_breaker_tripped_in_dag() -> Result<()> {
    // DAG: 3 tasks, dependencies [0]→[1]→[2], waves: [[0], [1], [2]]
    // - Wave 1: t1 fails twice (2 failures)
    // - Wave 2: t2 skipped (dependency failed) → 1 more failure = 3 → BREAKER TRIPS
    // Responses (FIFO): plan + 2 failures from t1 = 3 total
    let responses = vec![
        ok(r#"[{"task": "research", "deps": []}, {"task": "analyze", "deps": [0]}, {"task": "summarize", "deps": [1]}]"#), // plan: 3 tasks
        err("boom"), // t1 attempt 1: failure 1
        err("boom"), // t1 attempt 2: failure 2 → breaker would trip but let's continue
    ];
    let config = PatternConfig { circuit_breaker: 3, max_retries: 1, ..test_config() }; // 2 attempts per task
    let runner = Arc::new(MockRunner::new(responses));
    let (ctx, _rx) = make_ctx(config, runner.clone(), CancellationToken::new());

    let out = SwarmPattern::dag().execute(&ctx, "dag failure").await?;

    assert!(out.circuit_breaker_tripped, "circuit breaker should trip in DAG variant");
    Ok(())
}
