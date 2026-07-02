//! Layer 2 tests: quit shortcuts take priority over active turns.
//!
//! UiActor::handle_event_inner now returns `true` for Quit/ForceQuit
//! before routing to InputActor, so Ctrl+C and Ctrl+Q can exit even
//! when a turn is active.

use std::sync::Arc;

use runie_core::actors::leader::{LeaderAgentCmd, LeaderAgentHandle};
use runie_core::Event;

use crate::ui_actor::UiActor;
use crate::ui_actor_agent_handles::AgentHandleBox;

/// Minimal mock agent handle for testing.
struct MockAgentHandle {
    #[allow(dead_code)]
    run_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl MockAgentHandle {
    fn new() -> (Arc<Self>, tokio::sync::mpsc::Receiver<LeaderAgentCmd>) {
        let (_tx, rx) = tokio::sync::mpsc::channel(16);
        (
            Arc::new(Self {
                run_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            }),
            rx,
        )
    }
}

impl LeaderAgentHandle for MockAgentHandle {
    fn run(
        &self,
        _cmd: LeaderAgentCmd,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
    {
        self.run_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async {})
    }
    fn abort(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
}

fn make_ui_actor() -> (UiActor, Arc<MockAgentHandle>) {
    let (agent, _rx) = MockAgentHandle::new();
    let agent_arc = Arc::new(MockAgentHandle {
        run_count: agent.run_count.clone(),
    });
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let state = runie_core::AppState::default();
    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let bus = runie_core::bus::EventBus::<Event>::new(16);
    let (shutdown_tx, _) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    // Pass None for turn_handle and input_handle since these tests don't need them.
    let ui = UiActor::with_agent_handle(
        state,
        AgentHandleBox::Leader(agent_handle),
        None,
        None,
        kb_tx,
        bus,
        shutdown_tx,
        caps,
    );
    (ui, agent_arc)
}

/// Layer 2: Ctrl+C (Quit) exits even when a turn is active.
/// Quit/ForceQuit always return true, regardless of turn state.
#[tokio::test]
async fn ctrl_c_quits_during_turn() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Ctrl+C should return true (quit) regardless of active turn
    let quit = ui
        .handle_event_inner(Event::Quit {}, effect_tx.clone())
        .await;
    assert!(
        quit,
        "Quit event must return true even when a turn is active"
    );
}

/// Layer 2: Ctrl+Q (ForceQuit) exits immediately even when a turn is active.
#[tokio::test]
async fn ctrl_q_force_quits_during_turn() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Ctrl+Q should return true (quit) regardless of active turn
    let quit = ui
        .handle_event_inner(Event::ForceQuit {}, effect_tx.clone())
        .await;
    assert!(
        quit,
        "ForceQuit event must return true even when a turn is active"
    );
}

/// Layer 2: Ctrl+C does NOT exit when no turn is active (normal case).
#[tokio::test]
async fn ctrl_c_normal_idle_quits() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // No active turn — Ctrl+C should still quit
    let quit = ui
        .handle_event_inner(Event::Quit {}, effect_tx.clone())
        .await;
    assert!(quit, "Quit event must return true in idle state too");
}

/// Layer 2: Ctrl+S (Abort) aborts the active turn and returns to idle.
#[tokio::test]
async fn ctrl_s_aborts_during_turn() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Send TurnStarted to set active_request_id (simulating an active turn)
    ui.handle_event_inner(
        Event::TurnStarted {
            id: "test-id".to_string(),
            request_id: "test-request".to_string(),
            content: "test".to_string(),
        },
        effect_tx.clone(),
    )
    .await;
    ui.state.agent_state_mut().turn_active = true;

    // Ctrl+S should NOT return true (it's an abort, not a quit)
    let quit = ui
        .handle_event_inner(Event::Abort, effect_tx.clone())
        .await;
    assert!(!quit, "Abort event must NOT return true (it's not a quit)");

    // The turn should be aborted (turn_active cleared)
    assert!(
        !ui.state.agent_state().turn_active,
        "turn_active should be cleared after Abort"
    );

    // active_request_id should be cleared after Abort
    assert!(
        !ui.agent_running(),
        "agent_running should be cleared after Abort"
    );
}

/// Layer 2: Abort during idle state (no active turn) should still work.
#[tokio::test]
async fn abort_during_idle() {
    let (mut ui, _agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // No active turn — Abort should not quit
    let quit = ui
        .handle_event_inner(Event::Abort, effect_tx.clone())
        .await;
    assert!(!quit, "Abort event must not return true even in idle state");
}
