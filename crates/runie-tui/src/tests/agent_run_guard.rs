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
    fn abort(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
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
    let handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(agent.clone()));

    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let ui = UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(handle),
        None,
        None,
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
    assert!(
        ui.agent_running(),
        "guard should be active after TurnStarted"
    );

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

/// Layer 2: Done does NOT clear the guard (only TurnCompleted does).
/// This prevents a queued TurnStarted from bypassing the guard after Done.
#[tokio::test]
async fn done_does_not_clear_guard() {
    let (mut ui, agent) = make_ui_with_test_agent();
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
    assert_eq!(agent.run_count.load(std::sync::atomic::Ordering::SeqCst), 1);

    // Done: does NOT clear guard — guard stays true
    ui.handle_event(Event::Done { id: "req.0".into() }, effect_tx.clone())
        .await;
    assert!(
        ui.agent_running(),
        "agent_running stays true after Done — guard not cleared"
    );

    // TurnCompleted: finally clears guard
    ui.handle_event(Event::TurnCompleted, effect_tx.clone())
        .await;
    assert!(
        !ui.agent_running(),
        "agent_running should be false after TurnCompleted"
    );

    // Next TurnStarted: agent runs again
    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.1".into(),
            content: "second".into(),
            id: "req.1".into(),
        },
        effect_tx.clone(),
    )
    .await;

    assert_eq!(
        agent.run_count.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "agent should run again after guard is cleared by TurnCompleted"
    );
}

/// Layer 2: The exact bug scenario — Done + queued TurnStarted must not spawn
/// a second agent. After TurnStarted and Done, agent_running stays true so that
/// a TurnStarted from run_if_queued is blocked by the guard.
/// Only TurnCompleted clears the guard and allows the next real turn.
#[tokio::test]
async fn done_then_queued_turn_started_blocked_by_guard() {
    let (mut ui, agent) = make_ui_with_test_agent();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // TurnStarted for "hello" — agent spawned
    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.0".into(),
            content: "hello".into(),
            id: "req.0".into(),
        },
        effect_tx.clone(),
    )
    .await;
    assert_eq!(agent.run_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(ui.agent_running(), "guard active after TurnStarted");

    // Done arrives (as it does after the mock provider finishes)
    ui.handle_event(Event::Done { id: "req.0".into() }, effect_tx.clone())
        .await;
    // Guard must stay true — this is the fix! Done no longer clears it.
    assert!(
        ui.agent_running(),
        "guard stays true after Done to block queued TurnStarted"
    );

    // run_if_queued would emit TurnStarted(req.1, ...) here.
    // UiActor processes it — guard blocks the spawn.
    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.1".into(),
            content: "hello".into(),
            id: "req.1".into(),
        },
        effect_tx.clone(),
    )
    .await;

    // Still exactly one agent invocation — the queued TurnStarted was blocked
    assert_eq!(
        agent.run_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "queued TurnStarted must be blocked by guard after Done"
    );

    // TurnCompleted clears the guard
    ui.handle_event(Event::TurnCompleted, effect_tx.clone())
        .await;
    assert!(!ui.agent_running(), "guard cleared after TurnCompleted");

    // Next real user message: agent runs again
    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.2".into(),
            content: "world".into(),
            id: "req.2".into(),
        },
        effect_tx.clone(),
    )
    .await;
    assert_eq!(
        agent.run_count.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "next TurnStarted must spawn agent after guard cleared"
    );
}

/// Integration test (Layer 2+4): TurnActor publishes TurnStarted to the shared
/// Leader bus, and UiActor receives it and spawns the agent exactly once.
#[tokio::test]
async fn turn_actor_turn_started_reaches_uiactor_via_shared_bus() {
    use runie_core::actors::turn::RactorTurnActor;
    use runie_core::bus::EventBus;

    // Shared bus = Leader's event bus
    let bus = EventBus::<Event>::new(16);

    // Spawn UiActor with the shared bus
    let agent = TestAgentHandle::new();
    let handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(agent.clone()));
    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();
    let mut ui = crate::ui_actor::UiActor::with_agent_handle(
        state,
        crate::ui_actor_agent_handles::AgentHandleBox::Leader(handle),
        None,
        None,
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );

    // Spawn TurnActor using the SAME shared bus
    let (_turn_handle, _turn_cell, _turn_join) = RactorTurnActor::spawn(bus.clone()).await.unwrap();

    // Subscribe UiActor to the shared bus
    let mut bus_rx = bus.subscribe();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Send SubmitUserMessage to TurnActor via its handle
    // (TurnActor publishes TurnStarted to the shared bus)
    use runie_core::actors::turn::messages::MessageSource;
    use runie_core::actors::TurnMsg;
    let turn_handle = _turn_handle;
    turn_handle
        .send(TurnMsg::SubmitUserMessage {
            content: "hello".into(),
            id: "req.0".into(),
            source: MessageSource::Fresh,
        })
        .await;
    turn_handle.send(TurnMsg::RunIfQueued).await;

    // Give the actor time to process and publish TurnStarted
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Drain the bus until we see TurnStarted or timeout
    let mut found_turn_started = false;
    let deadline = tokio::time::sleep(std::time::Duration::from_millis(500));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            result = bus_rx.recv() => {
                if matches!(result, Ok(Event::TurnStarted { .. })) {
                    found_turn_started = true;
                }
                // Keep draining until we find TurnStarted or timeout
                if found_turn_started {
                    break;
                }
            }
            _ = &mut deadline => {
                break;
            }
        }
    }
    assert!(
        found_turn_started,
        "TurnActor must publish TurnStarted to the shared bus"
    );

    // Now process the TurnStarted event through UiActor
    let evt = Event::TurnStarted {
        request_id: "req.0".into(),
        content: "hello".into(),
        id: "req.0".into(),
    };
    ui.handle_event(evt, effect_tx.clone()).await;

    // Agent was spawned exactly once
    assert_eq!(
        agent.run_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "UiActor must spawn agent once when TurnStarted arrives via shared bus"
    );
}

/// Layer 2: Done from the shared bus does NOT clear agent_running.
/// Only TurnCompleted clears the guard.
#[tokio::test]
async fn done_from_shared_bus_does_not_clear_guard() {
    use runie_core::bus::EventBus;

    let bus = EventBus::<Event>::new(16);
    let agent = TestAgentHandle::new();
    let handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(Arc::new(agent.clone()));
    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();
    let mut ui = crate::ui_actor::UiActor::with_agent_handle(
        state,
        crate::ui_actor_agent_handles::AgentHandleBox::Leader(handle),
        None,
        None,
        kb_tx,
        bus,
        shutdown_tx,
        caps,
    );
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Simulate TurnStarted arriving via shared bus
    ui.handle_event(
        Event::TurnStarted {
            request_id: "req.0".into(),
            content: "hello".into(),
            id: "req.0".into(),
        },
        effect_tx.clone(),
    )
    .await;

    assert!(ui.agent_running(), "guard must be active after TurnStarted");

    // Simulate Done arriving via shared bus — guard stays true
    ui.handle_event(Event::Done { id: "req.0".into() }, effect_tx.clone())
        .await;

    assert!(
        ui.agent_running(),
        "guard must stay true after Done — only TurnCompleted clears it"
    );

    // TurnCompleted: clears guard
    ui.handle_event(Event::TurnCompleted, effect_tx.clone())
        .await;
    assert!(
        !ui.agent_running(),
        "guard must be cleared after TurnCompleted"
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

    assert!(
        ui.agent_running(),
        "guard should be active after TurnStarted"
    );

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
