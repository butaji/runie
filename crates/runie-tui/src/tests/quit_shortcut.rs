//! Layer 2 tests: quit / abort shortcuts vs. active turns.
//!
//! When a turn is in flight, Ctrl+C (Quit) and Esc (DialogBack at the chat
//! root) must ABORT the in-flight turn (via `agent_handle.abort()`) and keep
//! the app open — NOT quit. Ctrl+Q (ForceQuit) is the "really exit" hatch and
//! always quits. Idle Ctrl+C still quits.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use runie_core::actors::leader::{LeaderAgentCmd, LeaderAgentHandle};
use runie_core::Event;

use crate::ui_actor::UiActor;
use crate::ui_actor_agent_handles::AgentHandleBox;

/// Minimal mock agent handle for testing.
struct MockAgentHandle {
    #[allow(dead_code)]
    run_count: Arc<AtomicUsize>,
    abort_count: Arc<AtomicUsize>,
}

impl MockAgentHandle {
    fn new() -> (Arc<Self>, tokio::sync::mpsc::Receiver<LeaderAgentCmd>) {
        let (_tx, rx) = tokio::sync::mpsc::channel(16);
        (
            Arc::new(Self {
                run_count: Arc::new(AtomicUsize::new(0)),
                abort_count: Arc::new(AtomicUsize::new(0)),
            }),
            rx,
        )
    }

    fn aborts(&self) -> usize {
        self.abort_count.load(Ordering::SeqCst)
    }
}

impl LeaderAgentHandle for MockAgentHandle {
    fn run(
        &self,
        _cmd: LeaderAgentCmd,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        self.run_count.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {})
    }
    fn abort(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        self.abort_count.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {})
    }
}

fn make_ui_actor() -> (UiActor, Arc<MockAgentHandle>) {
    let (agent, _rx) = MockAgentHandle::new();
    let agent_arc = Arc::new(MockAgentHandle {
        run_count: agent.run_count.clone(),
        abort_count: agent.abort_count.clone(),
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

/// Drive UiActor into an active-turn state (turn_active + turn_was_active set).
async fn start_turn(ui: &mut UiActor, effect_tx: tokio::sync::mpsc::Sender<Event>) {
    ui.handle_event_inner(
        Event::TurnStarted {
            id: "test-id".to_string(),
            request_id: "test-request".to_string(),
            content: "test".to_string(),
        },
        effect_tx,
    )
    .await;
    ui.state.agent_state_mut().turn_active = true;
}

/// Layer 2: Ctrl+C (Quit) during an active turn ABORTS the turn and stays open.
#[tokio::test]
async fn ctrl_c_aborts_during_turn() {
    let (mut ui, agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);
    start_turn(&mut ui, effect_tx.clone()).await;

    let quit = ui
        .handle_event_inner(Event::Quit {}, effect_tx.clone())
        .await;

    assert!(
        !quit,
        "Quit (Ctrl+C) during an active turn must NOT quit the app"
    );
    assert_eq!(
        agent.aborts(),
        1,
        "Quit during an active turn must call agent_handle.abort() exactly once"
    );
    assert!(
        !ui.state.agent_state().turn_active,
        "turn_active should be cleared after Ctrl+C abort"
    );
    assert!(
        !ui.agent_running(),
        "agent_running should be cleared after Ctrl+C abort"
    );
}

/// Layer 2: Ctrl+Q (ForceQuit) always quits, even during an active turn.
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

/// Layer 2: Ctrl+C quits when idle and does NOT call abort.
#[tokio::test]
async fn ctrl_c_normal_idle_quits() {
    let (mut ui, agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // No active turn — Ctrl+C should quit
    let quit = ui
        .handle_event_inner(Event::Quit {}, effect_tx.clone())
        .await;
    assert!(quit, "Quit event must return true in idle state too");
    assert_eq!(
        agent.aborts(),
        0,
        "Idle Ctrl+C must not call agent_handle.abort()"
    );
}

/// Layer 2: Ctrl+S (Abort) aborts the active turn, cancels the agent, returns to idle.
#[tokio::test]
async fn ctrl_s_aborts_during_turn() {
    let (mut ui, agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);

    // Send TurnStarted to set active_request_id (simulating an active turn)
    start_turn(&mut ui, effect_tx.clone()).await;

    // Ctrl+S should NOT return true (it's an abort, not a quit)
    let quit = ui.handle_event_inner(Event::Abort, effect_tx.clone()).await;
    assert!(!quit, "Abort event must NOT return true (it's not a quit)");

    // Abort must cancel the in-flight agent exactly once.
    assert_eq!(
        agent.aborts(),
        1,
        "Abort during a turn must call agent_handle.abort() exactly once"
    );

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

    // No active turn — Abort should not quit (abort on idle is harmless).
    let quit = ui.handle_event_inner(Event::Abort, effect_tx.clone()).await;
    assert!(!quit, "Abort event must not return true even in idle state");
}

/// Layer 2: Esc (DialogBack at the chat root) during an active turn ABORTS the
/// turn and stays open.
#[tokio::test]
async fn esc_aborts_during_turn() {
    let (mut ui, agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);
    start_turn(&mut ui, effect_tx.clone()).await;
    assert!(ui.state.open_dialog().is_none(), "no dialog should be open");

    let quit = ui
        .handle_event_inner(Event::DialogBack, effect_tx.clone())
        .await;

    assert!(
        !quit,
        "Esc (DialogBack) during an active turn must NOT quit the app"
    );
    assert_eq!(
        agent.aborts(),
        1,
        "Esc during an active turn must call agent_handle.abort() exactly once"
    );
    assert!(
        !ui.state.agent_state().turn_active,
        "turn_active should be cleared after Esc abort"
    );
}

/// Layer 2: idle Esc (default non-vim, no dialog) is a no-op: no quit, no abort.
#[tokio::test]
async fn esc_idle_does_not_quit_or_abort() {
    let (mut ui, agent) = make_ui_actor();
    let (effect_tx, _effect_rx) = tokio::sync::mpsc::channel(16);
    assert!(ui.state.open_dialog().is_none(), "no dialog should be open");

    let quit = ui
        .handle_event_inner(Event::DialogBack, effect_tx.clone())
        .await;

    assert!(!quit, "Idle Esc must not quit the app");
    assert_eq!(
        agent.aborts(),
        0,
        "Idle Esc must not call agent_handle.abort()"
    );
}
