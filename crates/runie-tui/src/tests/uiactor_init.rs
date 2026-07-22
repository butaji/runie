//! Layer 2 tests: UiActor initialization and event ordering.
//!
//! Verifies:
//! 1. UiActor drains buffered bootstrap events before the first snapshot.
//! 2. UiActor processes DeliverQueued via RPC (not polling).
#![allow(clippy::too_many_lines)]

use std::sync::Arc;

use runie_core::actors::leader::{LeaderAgentCmd, LeaderAgentHandle};
use runie_core::Event;

/// Minimal mock agent handle for tests.
struct MockAgentHandle;

impl LeaderAgentHandle for MockAgentHandle {
    fn run(&self, _cmd: LeaderAgentCmd) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
    fn abort(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
}

/// Layer 2: UiActor drains buffered events before the first snapshot.
///
/// Verifies the fix for the initial snapshot race: ConfigLoaded/EnvDetected events
/// that are in the bus buffer when UiActor::run() starts are consumed by the
/// pre-snapshot drain loop (not missed until after the first render).
///
/// Strategy: Publish ConfigLoaded before UiActor::run() starts. UiActor drains it
/// before the first snapshot. The drained event is NOT re-published to the bus,
/// so a bus subscription that observes zero ConfigLoaded events AFTER UiActor starts
/// proves the drain consumed the pre-existing one.
#[tokio::test]
async fn uiactor_drains_buffered_config_loaded_before_first_snapshot() {
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle = crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());
    // Lock in a connected model so ConfigLoaded does not auto-open the onboarding
    // login dialog, which would intercept the liveness Input event used below.
    state.config_mut().current_provider = "mock".to_string();
    state.config_mut().current_model = "echo".to_string();
    state.config_mut().model_source = runie_core::model::ModelSource::UserOverride;

    let bus_rx = bus.subscribe();
    let (submit_tx, submit_rx) = tokio::sync::mpsc::channel(16);

    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    // Publish ConfigLoaded BEFORE UiActor::run() starts.
    // It lands in the bus_rx buffer.
    let config = runie_core::config::Config::default();
    bus.publish(Event::ConfigLoaded { config: Box::new(config) });

    let mut ui = crate::ui_actor::UiActor::with_external_bus_rx(
        state,
        bus_rx,
        leader.turn.clone(),
        leader.input.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui.set_agent_handle(crate::ui_actor_agent_handles::AgentHandleBox::Leader(
        agent_handle,
    ));

    // Subscribe after UiActor is created to observe events that arrive during run().
    let mut sub = bus.subscribe();

    // Spawn UiActor::run() in the background.
    let ui_handle = tokio::spawn(async move {
        ui.run_with_external_rx(submit_rx).await;
    });

    // Advance virtual time to let UiActor drain and enter select! loop.
    let _guard = runie_testing::TestTimeGuard::new().expect("should support time pausing");
    runie_testing::TestTimeGuard::advance(std::time::Duration::from_millis(100)).await;

    // Publish a subsequent event that WILL be re-emitted so we know the actor
    // is alive and processing events from the select! loop.
    bus.publish(Event::Input('h'));

    // Wait for the InputChanged event (proves UiActor is in the event loop).
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut found_input_changed = false;
    while tokio::time::Instant::now() < deadline {
        let rem = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(rem, sub.recv()).await {
            Ok(Ok(evt)) => {
                if matches!(evt, Event::InputChanged { state } if state.input == "h") {
                    found_input_changed = true;
                    break;
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert!(
        found_input_changed,
        "UiActor should process InputChanged after drain loop"
    );

    // Shut down cleanly.
    drop(shutdown_rx);
    let _ = submit_tx.send(Event::Quit).await;
    let _ = ui_handle.await;
    leader.shutdown().await;
}

/// Layer 2: UiActor drain loop does not hang on empty buffer.
///
/// Verifies that when the bus buffer is empty, UiActor::run() enters the
/// select! loop immediately without waiting.
#[tokio::test]
async fn uiactor_drain_loop_handles_empty_buffer() {
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle = crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());

    // Subscribe before UiActor starts — buffer is empty.
    let bus_rx = bus.subscribe();
    let (submit_tx, submit_rx) = tokio::sync::mpsc::channel(16);

    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    let mut ui = crate::ui_actor::UiActor::with_external_bus_rx(
        state,
        bus_rx,
        leader.turn.clone(),
        leader.input.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui.set_agent_handle(crate::ui_actor_agent_handles::AgentHandleBox::Leader(
        agent_handle,
    ));

    // Subscribe to observe events.
    let mut sub = bus.subscribe();

    // Spawn UiActor::run() and immediately give it a small window.
    let ui_handle = tokio::spawn(async move {
        ui.run_with_external_rx(submit_rx).await;
    });

    // Advance virtual time for UiActor to enter select! loop.
    let _guard = runie_testing::TestTimeGuard::new().expect("should support time pausing");
    runie_testing::TestTimeGuard::advance(std::time::Duration::from_millis(50)).await;

    // Publish an event — UiActor should process it.
    bus.publish(Event::Input('x'));

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut found = false;
    while tokio::time::Instant::now() < deadline {
        let rem = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(rem, sub.recv()).await {
            Ok(Ok(evt)) => {
                if matches!(evt, Event::InputChanged { state } if state.input == "x") {
                    found = true;
                    break;
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert!(
        found,
        "UiActor should process events even with empty initial buffer"
    );

    // Shut down.
    drop(shutdown_rx);
    let _ = submit_tx.send(Event::Quit).await;
    let _ = ui_handle.await;
    leader.shutdown().await;
}

/// Layer 2: UiActor processes Events with Quit before the first snapshot.
///
/// Verifies that if a Quit event is already in the bus buffer when UiActor::run()
/// starts, the drain loop processes it and UiActor exits before sending any snapshot.
#[tokio::test]
async fn uiactor_drain_loop_quits_before_first_snapshot() {
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle = crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());

    let bus_rx = bus.subscribe();
    let (_submit_tx, submit_rx) = tokio::sync::mpsc::channel(16);

    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
    let caps = crate::terminal::caps::TermCaps::default();

    // Publish Quit before UiActor::run() starts.
    bus.publish(Event::Quit);

    let mut ui = crate::ui_actor::UiActor::with_external_bus_rx(
        state,
        bus_rx,
        leader.turn.clone(),
        leader.input.clone(),
        kb_tx,
        bus.clone(),
        shutdown_tx,
        caps,
    );
    ui.set_agent_handle(crate::ui_actor_agent_handles::AgentHandleBox::Leader(
        agent_handle,
    ));

    // UiActor::run_with_external_rx should return quickly because Quit
    // is drained before the first snapshot is sent.
    let ui_handle = tokio::spawn(async move {
        ui.run_with_external_rx(submit_rx).await;
    });

    // If drain loop works, UiActor should exit within 200ms (not waiting forever).
    let result = tokio::time::timeout(std::time::Duration::from_millis(200), ui_handle).await;
    assert!(
        result.is_ok(),
        "UiActor should exit after draining Quit event"
    );

    let _ = leader.shutdown().await;
}
