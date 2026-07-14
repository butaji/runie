//! Black-box tests for runie-patterns Phase 3: EvalOptimizerPattern.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use runie_patterns::{
    AgentTrace, Context, EvalOptimizerPattern, Pattern, PatternConfig, PatternOutput,
    PatternRegistry, TerminationReason, WorkerRunner, WorkerTask,
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
/// sleep, optional abort-token cancellation on every call.
#[derive(Default)]
struct MockRunner {
    calls: Arc<Mutex<Vec<WorkerTask>>>,
    responses: Arc<Mutex<VecDeque<Result<String, String>>>>,
    sleep: Option<Duration>,
    abort_on_call: Option<CancellationToken>,
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

    fn with_abort_on_call(mut self, token: CancellationToken) -> Self {
        self.abort_on_call = Some(token);
        self
    }

    fn calls(&self) -> Vec<WorkerTask> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl WorkerRunner for MockRunner {
    async fn run(&self, task: WorkerTask) -> Result<String> {
        if let Some(token) = &self.abort_on_call {
            token.cancel();
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
        active: "eval-optimizer".into(),
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
    let ctx = Context {
        config,
        models: vec![("mock".into(), "echo".into())],
        semaphore: Arc::new(Semaphore::new(3)),
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
async fn eval_optimizer_approved_on_first_review() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![ok("draft v1"), ok("APPROVED")]));
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = EvalOptimizerPattern.execute(&ctx, "write a poem").await?;

    assert_eq!(out.result, "draft v1");
    assert_eq!(out.termination, TerminationReason::Approved);
    assert_eq!(trace_ids(&out), vec!["eval-generate-1", "eval-review-1"]);
    assert_eq!(out.traces[0].description, "generate");
    assert_eq!(out.traces[0].output, "draft v1");
    assert_eq!(out.traces[1].description, "review round 1");
    assert_eq!(out.traces[1].output, "APPROVED");

    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].prompt.lines().next(), Some("[eval-generate]"));
    assert!(calls[0].prompt.contains("write a poem"));
    assert_eq!(calls[1].prompt.lines().next(), Some("[eval-review]"));
    assert!(calls[1].prompt.contains("write a poem"));
    assert!(calls[1].prompt.contains("draft v1"));
    assert!(
        calls.iter().all(|t| t.read_only),
        "eval calls are read-only"
    );
    assert!(
        calls
            .iter()
            .all(|t| t.provider == "mock" && t.model == "echo"),
        "eval calls use the leader model"
    );
    Ok(())
}

#[tokio::test]
async fn eval_optimizer_feedback_then_approval() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![
        ok("draft v1"),
        ok("add more detail"),
        ok("draft v2"),
        ok("approved — looks good"), // case-insensitive approval
    ]));
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = EvalOptimizerPattern.execute(&ctx, "the task").await?;

    assert_eq!(out.result, "draft v2");
    assert_eq!(out.termination, TerminationReason::Approved);
    assert_eq!(
        trace_ids(&out),
        vec![
            "eval-generate-1",
            "eval-review-1",
            "eval-revise-2",
            "eval-review-2"
        ]
    );
    assert_eq!(out.traces[2].description, "revise round 2");

    let calls = runner.calls();
    assert_eq!(calls.len(), 4);
    let revise = &calls[2];
    assert_eq!(revise.id, "eval-revise-2");
    assert_eq!(revise.prompt.lines().next(), Some("[eval-revise]"));
    assert!(revise.prompt.contains("the task"));
    assert!(
        revise.prompt.contains("draft v1"),
        "revise prompt carries the current draft"
    );
    assert!(
        revise.prompt.contains("add more detail"),
        "revise prompt carries the reviewer feedback"
    );
    Ok(())
}

#[tokio::test]
async fn eval_optimizer_max_rounds_without_approval() -> Result<()> {
    let config = PatternConfig {
        max_rounds: 2,
        ..test_config()
    };
    let runner = Arc::new(MockRunner::new(vec![
        ok("draft v1"),
        ok("feedback 1"),
        ok("draft v2"),
        ok("feedback 2"),
    ]));
    let (ctx, _rx) = make_ctx(config, runner.clone(), CancellationToken::new());

    let out = EvalOptimizerPattern
        .execute(&ctx, "never good enough")
        .await?;

    assert_eq!(out.termination, TerminationReason::MaxRoundsReached);
    assert_eq!(out.result, "draft v2", "result is the last draft");
    assert_eq!(
        trace_ids(&out),
        vec![
            "eval-generate-1",
            "eval-review-1",
            "eval-revise-2",
            "eval-review-2"
        ]
    );
    assert_eq!(runner.calls().len(), 4, "2 rounds x (draft + review)");
    Ok(())
}

#[tokio::test]
async fn eval_optimizer_abort_during_call() -> Result<()> {
    let abort = CancellationToken::new();
    let runner = Arc::new(
        MockRunner::new(vec![ok("never used")])
            .with_sleep(Duration::from_millis(300))
            .with_abort_on_call(abort.clone()),
    );
    let (ctx, _rx) = make_ctx(test_config(), runner, abort);
    let start = Instant::now();

    let out = EvalOptimizerPattern.execute(&ctx, "abort me").await?;

    assert_eq!(out.termination, TerminationReason::Error("aborted".into()));
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "abort must be fast"
    );
    Ok(())
}

#[tokio::test]
async fn eval_optimizer_generator_error_returns_empty_result() -> Result<()> {
    let runner = Arc::new(MockRunner::new(vec![err("boom")]));
    let (ctx, _rx) = make_ctx(test_config(), runner.clone(), CancellationToken::new());

    let out = EvalOptimizerPattern.execute(&ctx, "explode").await?;

    assert_eq!(out.result, "", "no draft exists yet");
    match &out.termination {
        TerminationReason::Error(msg) => assert!(msg.contains("boom"), "got {msg}"),
        other => panic!("expected Error termination, got {other:?}"),
    }
    assert_eq!(trace_ids(&out), vec!["eval-generate-1"]);
    Ok(())
}

#[test]
fn eval_optimizer_registry_metadata() {
    let registry = PatternRegistry::default();
    let pattern = registry
        .get("eval-optimizer")
        .expect("eval-optimizer registered");
    assert_eq!(pattern.name(), "eval-optimizer");
    assert_eq!(
        pattern.description(),
        "Critical review loops — generate, evaluate, revise"
    );

    let names = registry.names();
    let swarm_pos = names
        .iter()
        .position(|n| *n == "swarm")
        .expect("swarm registered");
    let eval_pos = names
        .iter()
        .position(|n| *n == "eval-optimizer")
        .expect("eval-optimizer registered");
    assert!(
        eval_pos > swarm_pos,
        "eval-optimizer is registered after swarm"
    );
}
