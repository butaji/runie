//! Layer 2 tests: pattern-mode turn interception (PATTERNS.md Phase 2-3).
//!
//! A `TurnStarted` with `[mode].active == "swarm"` or `"eval-optimizer"`
//! must NOT call the agent handle; instead UiActor runs the pattern and
//! publishes the same terminal events the agent actor would (Thinking,
//! Response, TurnComplete, Done) plus worker feed rows for swarm.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use runie_core::actors::leader::{LeaderAgentCmd, LeaderAgentHandle};
use runie_core::bus::{EventBus, Receiver};
use runie_core::Event;
use runie_patterns::{WorkerRunner, WorkerTask};

// ── Test doubles ─────────────────────────────────────────────────────────────

/// Agent handle that counts `run` invocations (mirrors agent_run_guard.rs).
#[derive(Clone, Default)]
struct TestAgentHandle {
    run_count: Arc<AtomicUsize>,
}

impl LeaderAgentHandle for TestAgentHandle {
    fn run(
        &self,
        _cmd: LeaderAgentCmd,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        self.run_count.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {})
    }
    fn abort(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
}

/// Worker runner that returns a fixed response instantly. With the swarm
/// pattern this yields: leader plan (unparseable → single fallback task) →
/// one worker → leader synthesis — all returning `output`.
struct EchoRunner {
    output: String,
}

#[async_trait::async_trait]
impl WorkerRunner for EchoRunner {
    async fn run(&self, _task: WorkerTask) -> anyhow::Result<String> {
        Ok(self.output.clone())
    }
}

/// Worker runner that never completes — only an abort unblocks the pattern.
struct PendingRunner;

#[async_trait::async_trait]
impl WorkerRunner for PendingRunner {
    async fn run(&self, _task: WorkerTask) -> anyhow::Result<String> {
        std::future::pending::<()>().await;
        unreachable!("pending runner only returns via abort")
    }
}

// ── Harness ──────────────────────────────────────────────────────────────────

fn make_ui(agent: &TestAgentHandle, bus: &EventBus<Event>) -> crate::ui_actor::UiActor {
    use crate::ui_actor::UiActor;
    use crate::ui_actor_agent_handles::{AgentHandleBox, LeaderAgentActorHandle};

    let handle = LeaderAgentActorHandle::new(Arc::new(agent.clone()));
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    UiActor::with_agent_handle(
        runie_core::AppState::default(),
        AgentHandleBox::Leader(handle),
        None,
        None,
        kb_tx,
        bus.clone(),
        shutdown_tx,
        crate::terminal::caps::TermCaps::default(),
    )
}

fn turn_started(id: &str, content: &str) -> Event {
    Event::TurnStarted {
        request_id: id.into(),
        content: content.into(),
        id: id.into(),
    }
}

/// Drain bus events until `stop` matches, bounded by a deadline so a missing
/// event fails fast instead of hanging. Returns everything received.
async fn drain_until(bus_rx: &mut Receiver<Event>, stop: impl Fn(&Event) -> bool) -> Vec<Event> {
    let mut seen = Vec::new();
    let deadline = tokio::time::sleep(std::time::Duration::from_secs(2));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            result = bus_rx.recv() => {
                match result {
                    Ok(evt) => {
                        let hit = stop(&evt);
                        seen.push(evt);
                        if hit {
                            return seen;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            _ = &mut deadline => break,
        }
    }
    seen
}

/// Let spawned tasks run to quiescence, then drain whatever is on the bus.
async fn drain_available(bus_rx: &mut Receiver<Event>) -> Vec<Event> {
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    let mut out = Vec::new();
    while let Ok(evt) = bus_rx.try_recv() {
        out.push(evt);
    }
    out
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Swarm mode: the pattern replaces the agent turn and publishes the full
/// terminal event contract (Thinking → worker rows → Response → TurnComplete
/// → Done) so the TurnActor finalizes the turn exactly like an agent turn.
#[tokio::test]
async fn swarm_turn_runs_pattern_not_agent() {
    let bus = EventBus::<Event>::new(16);
    let mut bus_rx = bus.subscribe();
    let agent = TestAgentHandle::default();
    let mut ui = make_ui(&agent, &bus);
    ui.set_pattern_executor(Arc::new(EchoRunner {
        output: "pattern-result".into(),
    }));
    ui.state.config_mut().mode.active = "swarm".into();

    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);
    ui.handle_event(turn_started("req.0", "hello"), effect_tx.clone())
        .await;

    assert_eq!(
        agent.run_count.load(Ordering::SeqCst),
        0,
        "swarm turn must not spawn the agent"
    );
    assert!(ui.agent_running(), "guard active while pattern runs");

    let events = drain_until(
        &mut bus_rx,
        |e| matches!(e, Event::Done { id } if id == "req.0"),
    )
    .await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Thinking { id } if id == "req.0")),
        "thinking row must show for the whole pattern run: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::PatternWorkerSpawned { id, .. } if id == "worker-1-0")),
        "worker spawn row must be published: {events:?}"
    );
    assert!(
        events.iter().any(
            |e| matches!(e, Event::PatternWorkerFinished { id, status, .. } if id == "worker-1-0" && status == "completed")
        ),
        "worker finish row must be published: {events:?}"
    );
    assert!(
        events.iter().any(
            |e| matches!(e, Event::Response { id, content, .. } if id == "req.0" && content == "pattern-result")
        ),
        "final assistant response must be published: {events:?}"
    );
    let turn_complete = events
        .iter()
        .position(|e| matches!(e, Event::TurnComplete { id, .. } if id == "req.0"))
        .expect("TurnComplete must be published");
    let done = events
        .iter()
        .position(|e| matches!(e, Event::Done { id } if id == "req.0"))
        .expect("Done must be published");
    assert!(
        turn_complete < done,
        "TurnComplete must precede Done (agent contract)"
    );

    // TurnCompleted (emitted by the TurnActor bridge after Done) clears the
    // guard and the pattern state.
    ui.handle_event(Event::TurnCompleted, effect_tx.clone())
        .await;
    assert!(!ui.agent_running(), "guard cleared after TurnCompleted");
    assert!(
        ui.pattern_abort_token().is_none(),
        "pattern state cleared after the turn completes"
    );
}

/// eval-optimizer routes through the pattern engine: no agent spawn, and
/// the standard terminal contract (Thinking, Response, TurnComplete, Done).
/// EchoRunner never returns "APPROVED", so the loop exhausts max_rounds and
/// delivers the last draft as the result.
#[tokio::test]
async fn eval_optimizer_runs_through_pattern_engine() {
    let bus = EventBus::<Event>::new(16);
    let mut bus_rx = bus.subscribe();
    let agent = TestAgentHandle::default();
    let mut ui = make_ui(&agent, &bus);
    ui.set_pattern_executor(Arc::new(EchoRunner { output: "x".into() }));
    ui.state.config_mut().mode.active = "eval-optimizer".into();

    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);
    ui.handle_event(turn_started("req.0", "hello"), effect_tx.clone())
        .await;

    assert_eq!(
        agent.run_count.load(Ordering::SeqCst),
        0,
        "eval-optimizer must not spawn the agent"
    );

    let events = drain_until(
        &mut bus_rx,
        |e| matches!(e, Event::Done { id } if id == "req.0"),
    )
    .await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Thinking { id } if id == "req.0")),
        "thinking row must show for the whole pattern run: {events:?}"
    );
    assert!(
        events.iter().any(
            |e| matches!(e, Event::Response { id, content, .. } if id == "req.0" && content == "x")
        ),
        "last draft must be published as the response: {events:?}"
    );
    let turn_complete = events
        .iter()
        .position(|e| matches!(e, Event::TurnComplete { id, .. } if id == "req.0"))
        .expect("TurnComplete must be published");
    let done = events
        .iter()
        .position(|e| matches!(e, Event::Done { id } if id == "req.0"))
        .expect("Done must be published");
    assert!(turn_complete < done, "TurnComplete must precede Done");
}

/// Swarm mode without an installed runner falls back to the agent turn.
#[tokio::test]
async fn swarm_without_runner_falls_back_to_agent() {
    let bus = EventBus::<Event>::new(16);
    let agent = TestAgentHandle::default();
    let mut ui = make_ui(&agent, &bus);
    // No set_pattern_executor call.
    ui.state.config_mut().mode.active = "swarm".into();

    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);
    ui.handle_event(turn_started("req.0", "hello"), effect_tx.clone())
        .await;

    assert_eq!(
        agent.run_count.load(Ordering::SeqCst),
        1,
        "swarm without a runner must fall back to the agent turn"
    );
}

/// Abort (Esc, Ctrl+C, /new) cancels the in-flight pattern run; the pattern
/// task must not publish terminal events afterwards (the Abort event path
/// already finalized the turn).
#[tokio::test]
async fn abort_cancels_pattern_run_without_terminal_events() {
    let bus = EventBus::<Event>::new(16);
    let mut bus_rx = bus.subscribe();
    let agent = TestAgentHandle::default();
    let mut ui = make_ui(&agent, &bus);
    ui.set_pattern_executor(Arc::new(PendingRunner));
    ui.state.config_mut().mode.active = "swarm".into();

    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);
    ui.handle_event(turn_started("req.0", "hello"), effect_tx.clone())
        .await;
    let token = ui
        .pattern_abort_token()
        .expect("pattern abort token installed on pattern turn");

    ui.handle_event(Event::Abort, effect_tx.clone()).await;

    assert!(token.is_cancelled(), "abort must cancel the pattern run");
    assert!(!ui.agent_running(), "guard cleared after abort");
    assert!(
        ui.pattern_abort_token().is_none(),
        "pattern state taken on abort"
    );

    let events = drain_available(&mut bus_rx).await;
    assert!(
        events.iter().all(|e| !matches!(e, Event::Done { .. })),
        "pattern task must not publish Done after abort: {events:?}"
    );
    assert!(
        events.iter().all(|e| !matches!(e, Event::Response { .. })),
        "pattern task must not publish a response after abort: {events:?}"
    );
}
