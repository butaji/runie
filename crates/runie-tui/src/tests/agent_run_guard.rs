//! Layer 2 tests: UiActor agent run guard prevents duplicate agent spawns.
//!
//! This module tests that UiActor::handle_event_inner guards against duplicate
//! agent runs when TurnStarted arrives multiple times or when a queued
//! TurnStarted arrives before the previous agent completes.

use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

use runie_core::actors::leader::{LeaderAgentCmd, LeaderAgentHandle};
use runie_core::Event;

// ── Mock agent handle that counts runs ────────────────────────────────────────

struct MockAgentHandle {
    run_count: std::sync::atomic::AtomicUsize,
    run_tx: mpsc::Sender<LeaderAgentCmd>,
}

impl MockAgentHandle {
    fn new() -> (Self, mpsc::Receiver<LeaderAgentCmd>) {
        let (run_tx, run_rx) = mpsc::channel(16);
        (
            Self {
                run_count: std::sync::atomic::AtomicUsize::new(0),
                run_tx,
            },
            run_rx,
        )
    }
}

impl LeaderAgentHandle for MockAgentHandle {
    fn run(&self, cmd: LeaderAgentCmd) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let tx = self.run_tx.clone();
        self.run_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move {
            let _ = tx.send(cmd).await;
        })
    }
}

// ── Test helpers ────────────────────────────────────────────────────────────────

/// UiActor test harness: constructs a minimal UiActor with a mock agent handle.
#[derive(Clone)]
struct TestAgentHandle {
    run_count: Arc<std::sync::atomic::AtomicUsize>,
    run_rx: Arc<parking_lot::Mutex<Option<mpsc::Receiver<LeaderAgentCmd>>>>,
}

impl TestAgentHandle {
    fn new() -> (Self, mpsc::Receiver<LeaderAgentCmd>) {
        let (run_tx, run_rx) = mpsc::channel(16);
        (
            Self {
                run_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                run_rx: Arc::new(parking_lot::Mutex::new(Some(run_rx))),
            },
            run_rx,
        )
    }
}

impl LeaderAgentHandle for TestAgentHandle {
    fn run(
        &self,
        cmd: LeaderAgentCmd,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let tx = self.run_rx.lock().clone();
        self.run_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move {
            if let Some(ref rx) = *tx {
                let _ = rx.send(cmd).await;
            }
        })
    }
}

// ── Layer 2: Event handling tests ─────────────────────────────────────────────

#[tokio::test]
async fn turn_started_spawns_agent_once() {
    use crate::ui_actor::UiActor;
    use crate::ui_actor_agent_handles::AgentHandleBox;

    let (agent, _run_rx) = TestAgentHandle::new();
    let handle = crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(agent));

    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(handle),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );

    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // First TurnStarted: agent should run
    let evt = Event::TurnStarted {
        request_id: "req.0".into(),
        content: "hello".into(),
        id: "req.0".into(),
    };
    ui.handle_event(evt, effect_tx.clone()).await;

    // Agent was called exactly once
    let count = ui.agent_running_count();
    assert_eq!(count, 1, "agent should run once for first TurnStarted");
}

#[tokio::test]
async fn second_turn_started_blocked_by_guard() {
    use crate::ui_actor::UiActor;
    use crate::ui_actor_agent_handles::AgentHandleBox;

    let (agent, _run_rx) = TestAgentHandle::new();
    let handle = crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(agent));

    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(handle),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );

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
    let count = ui.agent_running_count();
    assert_eq!(count, 1, "second TurnStarted should be blocked by guard");
}

#[tokio::test]
async fn done_clears_guard_allows_next_turn() {
    use crate::ui_actor::UiActor;
    use crate::ui_actor_agent_handles::AgentHandleBox;

    let (agent, _run_rx) = TestAgentHandle::new();
    let handle = crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(agent));

    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(handle),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );

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

    // Done: clears guard
    ui.handle_event(Event::Done { id: "req.0".into() }, effect_tx.clone())
        .await;

    // Agent should NOT be running after Done
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
    let count = ui.agent_running_count();
    assert_eq!(count, 2, "agent should run again after Done clears guard");
}

#[tokio::test]
async fn turn_errored_clears_guard() {
    use crate::ui_actor::UiActor;
    use crate::ui_actor_agent_handles::AgentHandleBox;

    let (agent, _run_rx) = TestAgentHandle::new();
    let handle = crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(agent));

    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(handle),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );

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
