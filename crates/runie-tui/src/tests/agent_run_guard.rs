//! Layer 2 tests: UiActor agent run guard prevents duplicate agent spawns.
//!
//! This module tests that UiActor::handle_event_inner guards against duplicate
//! agent runs when TurnStarted arrives multiple times or when a queued
//! TurnStarted arrives before the previous agent completes.

use std::sync::Arc;

use runie_core::actors::leader::{LeaderAgentCmd, LeaderAgentHandle};
use runie_core::Event;

// ── Test harness ────────────────────────────────────────────────────────────────

/// UiActor test harness: constructs a minimal UiActor with a mock agent handle.
/// Commands are stored in a thread-safe Vec for inspection.
#[derive(Clone)]
struct TestAgentHandle {
    run_count: Arc<std::sync::atomic::AtomicUsize>,
    commands: Arc<parking_lot::Mutex<Vec<LeaderAgentCmd>>>,
}

impl TestAgentHandle {
    fn new() -> Self {
        Self {
            run_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            commands: Arc::new(parking_lot::Mutex::new(Vec::new())),
        }
    }
}

impl LeaderAgentHandle for TestAgentHandle {
    fn run(
        &self,
        cmd: LeaderAgentCmd,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        self.run_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.commands.lock().push(cmd);
        Box::pin(async {})
    }
}

// ── Layer 2: Event handling tests ─────────────────────────────────────────────

/// Helper to build a UiActor with a TestAgentHandle, returning both.
/// The returned `agent` is a clone that shares the run counter with the one
/// inside the UiActor, so tests can inspect the invocation count.
fn make_ui_with_test_agent() -> (crate::ui_actor::UiActor, TestAgentHandle) {
    use crate::ui_actor::UiActor;
    use crate::ui_actor_agent_handles::AgentHandleBox;

    let agent = TestAgentHandle::new();
    let handle = crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(agent.clone()));

    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let ui = UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(handle),
        kb_tx,
        bus,
        shutdown_tx,
        caps,
    );
    (ui, agent)
}

/// Layer 2: TurnStarted spawns the agent exactly once.
#[tokio::test]
async fn turn_started_spawns_agent_once() {
    let (mut ui, agent) = make_ui_with_test_agent();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // First TurnStarted: agent should run
    let evt = Event::TurnStarted {
        request_id: "req.0".into(),
        content: "hello".into(),
        id: "req.0".into(),
    };
    ui.handle_event(evt, effect_tx.clone()).await;

    // Agent handle recorded exactly one run
    assert_eq!(
        agent.run_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "agent should run once for first TurnStarted"
    );
}

/// Layer 2: A second TurnStarted while the guard is active is blocked.
#[tokio::test]
async fn second_turn_started_blocked_by_guard() {
    let (mut ui, agent) = make_ui_with_test_agent();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // First TurnStarted
    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.0".into(),
            content: "hello".into(),
            id: "req.0".into(),
        },
        effect_tx.clone(),
    )
    .await;

    // Guard is set
    assert!(ui.agent_running(), "guard should be active after TurnStarted");

    // Second TurnStarted (e.g., from queue delivery while first is still running)
    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.1".into(),
            content: "follow-up".into(),
            id: "req.1".into(),
        },
        effect_tx.clone(),
    )
    .await;

    // Agent was called exactly once (second blocked by guard)
    assert_eq!(
        agent.run_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "second TurnStarted should be blocked by guard"
    );
}

/// Layer 2: Done clears the guard and allows the next TurnStarted to run.
#[tokio::test]
async fn done_clears_guard_allows_next_turn() {
    let (mut ui, agent) = make_ui_with_test_agent();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // First TurnStarted
    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.0".into(),
            content: "hello".into(),
            id: "req.0".into(),
        },
        effect_tx.clone(),
    )
    .await;

    assert_eq!(
        agent.run_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "first TurnStarted should spawn agent"
    );

    // Done: clears guard
    ui.handle_event(Event::Done { id: "req.0".into() }, effect_tx.clone())
        .await;

    assert!(
        !ui.agent_running(),
        "agent_running should be false after Done"
    );

    // Next TurnStarted: agent should run again
    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.1".into(),
            content: "second".into(),
            id: "req.1".into(),
        },
        effect_tx.clone(),
    )
    .await;

    // Agent was called twice (once per turn)
    assert_eq!(
        agent.run_count.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "agent should run again after Done clears guard"
    );
}

/// Layer 2: TurnErrored clears the guard.
#[tokio::test]
async fn turn_errored_clears_guard() {
    let (mut ui, _agent) = make_ui_with_test_agent();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.0".into(),
            content: "hello".into(),
            id: "req.0".into(),
        },
        effect_tx.clone(),
    )
    .await;

    assert!(ui.agent_running(), "guard should be active after TurnStarted");

    ui.handle_event(
        Event::TurnErrored {
            id: "req.0".into(),
            message: "boom".into(),
        },
        effect_tx.clone(),
    )
    .await;

    assert!(
        !ui.agent_running(),
        "agent_running should be false after TurnErrored"
    );
}
