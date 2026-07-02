//! Layer 2 tests: `UiActor` routes `Event::Input` through `InputActor`.
//!
//! Ensures that input events go through the InputActor → InputChanged path,
//! not direct `AppState.input` mutation.

use std::sync::Arc;

use runie_core::actors::leader::LeaderAgentCmd;
use runie_core::actors::leader::LeaderAgentHandle;
use runie_core::Event;

/// Minimal mock agent handle for testing.
struct MockAgentHandle;

impl LeaderAgentHandle for MockAgentHandle {
    fn run(
        &self,
        _cmd: LeaderAgentCmd,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
    fn abort(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
}

/// Layer 2: `Event::Input(c)` sent to UiActor produces exactly one
/// `InputChanged` event (proving InputActor received the InsertChar message).
#[tokio::test]
async fn input_event_routes_to_input_actor() {
    // Build a LeaderHandle with all real actors (including InputActor).
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());

    // Subscribe before UiActor starts so we capture all events.
    let bus_rx = bus.subscribe();
    let (submit_tx, submit_rx) = tokio::sync::mpsc::channel(16);

    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
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

    // Second subscription to observe events.
    let mut sub = bus.subscribe();

    // Run UiActor in a background task.
    let ui_handle = tokio::spawn(async move {
        ui.run_with_external_rx(submit_rx).await;
    });

    // Give the actor a moment to start.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Send Input('h') through the submit channel.
    submit_tx
        .send(Event::Input('h'))
        .await
        .expect("submit channel open");

    // Wait for InputChanged event (InputActor emits it in response to InsertChar).
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
        "Expected InputChanged event with 'h' after Event::Input('h')"
    );

    // Quit to shut down the actor.
    submit_tx.send(Event::Quit).await.expect("submit open");
    let _ = ui_handle.await;

    leader.shutdown().await;
}

/// Layer 2: Multiple characters accumulate correctly through InputActor.
#[tokio::test]
async fn input_accumulates_via_input_actor() {
    let leader = runie_core::actors::leader::test_leader_handle().await;
    let bus = leader.event_bus().clone();

    let agent_arc: Arc<dyn LeaderAgentHandle> = Arc::new(MockAgentHandle);
    let agent_handle =
        crate::ui_actor_agent_handles::LeaderAgentActorHandle::new(agent_arc.clone());

    let mut state = runie_core::AppState::default();
    state.set_actor_handles(leader.clone());

    let bus_rx = bus.subscribe();
    let (submit_tx, submit_rx) = tokio::sync::mpsc::channel(16);

    let (kb_tx, _kb_rx) = tokio::sync::watch::channel(Default::default());
    let (shutdown_tx, _shutdown_rx) = tokio::sync::oneshot::channel();
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

    let mut sub = bus.subscribe();

    let ui_handle = tokio::spawn(async move {
        ui.run_with_external_rx(submit_rx).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Type "hi" character by character.
    submit_tx.send(Event::Input('h')).await.expect("submit open");
    submit_tx.send(Event::Input('i')).await.expect("submit open");

    // Wait for the final InputChanged event.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut final_input = String::new();
    while tokio::time::Instant::now() < deadline {
        let rem = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(rem, sub.recv()).await {
            Ok(Ok(evt)) => {
                if let Event::InputChanged { state } = evt {
                    final_input = state.input.clone();
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }
    assert_eq!(
        final_input, "hi",
        "Input should accumulate to 'hi' via InputActor routing"
    );

    // Quit to shut down the actor.
    submit_tx.send(Event::Quit).await.expect("submit open");
    let _ = ui_handle.await;

    leader.shutdown().await;
}
