//! Black-box tests for runie-patterns Phase 1: core primitives + SinglePattern.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use runie_patterns::{
    model_for, AgentTrace, Context, Pattern, PatternConfig, PatternRegistry, SinglePattern,
    TerminationReason, TraceEvent, WorkerRunner, WorkerTask,
};
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

/// Mock worker runner: records calls, returns a canned result, optionally sleeps.
struct MockRunner {
    calls: Arc<Mutex<Vec<WorkerTask>>>,
    canned: Arc<Mutex<Result<String, String>>>,
    sleep: Option<Duration>,
}

impl MockRunner {
    fn ok(text: &str) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            canned: Arc::new(Mutex::new(Ok(text.to_string()))),
            sleep: None,
        }
    }

    fn err(msg: &str) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            canned: Arc::new(Mutex::new(Err(msg.to_string()))),
            sleep: None,
        }
    }

    fn with_sleep(mut self, d: Duration) -> Self {
        self.sleep = Some(d);
        self
    }

    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait::async_trait]
impl WorkerRunner for MockRunner {
    async fn run(&self, task: WorkerTask) -> Result<String> {
        self.calls.lock().unwrap().push(task);
        if let Some(d) = self.sleep {
            tokio::time::sleep(d).await;
        }
        match &*self.canned.lock().unwrap() {
            Ok(text) => Ok(text.clone()),
            Err(msg) => Err(anyhow::anyhow!(msg.clone())),
        }
    }
}

fn leader_task() -> WorkerTask {
    WorkerTask {
        id: "leader".into(),
        prompt: "hello".into(),
        provider: "mock".into(),
        model: "echo".into(),
        read_only: false,
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

#[test]
fn pattern_config_default_values() {
    let cfg = PatternConfig::default();
    assert_eq!(cfg.active, "single");
    assert_eq!(cfg.workers, 3);
    assert_eq!(cfg.max_rounds, 5);
    assert_eq!(cfg.timeout_ms, 120_000);
    assert_eq!(cfg.max_retries, 2);
    assert_eq!(cfg.circuit_breaker, 3);
}

#[test]
fn model_for_leader_and_worker_fallback() {
    let models = vec![
        ("anthropic".to_string(), "claude".to_string()),
        ("openai".to_string(), "gpt".to_string()),
        ("mock".to_string(), "echo".to_string()),
    ];

    // Leader is always index 0.
    assert_eq!(
        model_for(&models, 0),
        ("anthropic".to_string(), "claude".to_string())
    );
    // Workers use the next models while available.
    assert_eq!(
        model_for(&models, 1),
        ("openai".to_string(), "gpt".to_string())
    );
    assert_eq!(model_for(&models, 2), ("mock".to_string(), "echo".to_string()));
    // Fewer models than workers: workers reuse the leader model.
    assert_eq!(
        model_for(&models, 3),
        ("anthropic".to_string(), "claude".to_string())
    );
    assert_eq!(
        model_for(&models, 10),
        ("anthropic".to_string(), "claude".to_string())
    );

    // Single model is reused for everyone.
    let one = vec![("mock".to_string(), "echo".to_string())];
    for i in 0..5 {
        assert_eq!(model_for(&one, i), ("mock".to_string(), "echo".to_string()));
    }
}

#[tokio::test]
async fn single_pattern_happy_path() -> Result<()> {
    let runner = Arc::new(MockRunner::ok("done"));
    let (ctx, mut trace_rx) = make_ctx(
        PatternConfig::default(),
        runner.clone(),
        CancellationToken::new(),
    );

    let out = SinglePattern.execute(&ctx, "hello").await?;

    assert_eq!(out.result, "done");
    assert_eq!(out.termination, TerminationReason::Completed);
    assert_eq!(out.traces.len(), 1);
    assert_eq!(out.traces[0].agent_id, "leader");
    assert_eq!(
        out.traces[0].events,
        vec![TraceEvent::Termination {
            reason: TerminationReason::Completed
        }]
    );

    // Trace was also delivered on the trace channel.
    let trace = trace_rx.try_recv().expect("trace delivered on channel");
    assert_eq!(trace.agent_id, "leader");

    // Runner was called exactly once with the leader task.
    assert_eq!(runner.call_count(), 1);
    assert_eq!(runner.calls.lock().unwrap()[0], leader_task());
    Ok(())
}

#[tokio::test]
async fn single_pattern_runner_error() -> Result<()> {
    let runner = Arc::new(MockRunner::err("boom"));
    let (ctx, _trace_rx) = make_ctx(
        PatternConfig::default(),
        runner.clone(),
        CancellationToken::new(),
    );

    let out = SinglePattern.execute(&ctx, "hello").await?;

    assert_eq!(out.result, "");
    match &out.termination {
        TerminationReason::Error(msg) => assert!(msg.contains("boom")),
        other => panic!("expected Error, got {other:?}"),
    }
    assert_eq!(out.traces.len(), 1);
    assert_eq!(runner.call_count(), 1);
    Ok(())
}

#[tokio::test]
async fn single_pattern_timeout() -> Result<()> {
    let runner = Arc::new(MockRunner::ok("too late").with_sleep(Duration::from_millis(500)));
    let config = PatternConfig {
        timeout_ms: 50,
        ..PatternConfig::default()
    };
    let (ctx, _trace_rx) = make_ctx(config, runner.clone(), CancellationToken::new());

    let out = SinglePattern.execute(&ctx, "hello").await?;

    assert_eq!(out.termination, TerminationReason::Timeout);
    assert_eq!(out.result, "");
    assert_eq!(runner.call_count(), 1);
    Ok(())
}

#[tokio::test]
async fn single_pattern_abort_before_execute() -> Result<()> {
    let runner = Arc::new(MockRunner::ok("done"));
    let abort = CancellationToken::new();
    abort.cancel();
    let (ctx, _trace_rx) = make_ctx(PatternConfig::default(), runner.clone(), abort);

    let out = SinglePattern.execute(&ctx, "hello").await?;

    assert_eq!(out.termination, TerminationReason::Error("aborted".into()));
    assert_eq!(out.result, "");
    // Runner must never be called once aborted.
    assert_eq!(runner.call_count(), 0);
    Ok(())
}

#[test]
fn registry_defaults_and_lookup() {
    let registry = PatternRegistry::default();

    assert!(registry.names().contains(&"single"));

    let single = registry.get("single").expect("single pattern registered");
    assert_eq!(single.name(), "single");
    assert!(!single.description().is_empty());

    assert!(registry.get("nope").is_none());

    // Empty registry + manual registration.
    let mut empty = PatternRegistry::new();
    assert!(empty.get("single").is_none());
    empty.register(Box::new(SinglePattern));
    assert!(empty.get("single").is_some());
    assert_eq!(empty.names(), vec!["single"]);
}
