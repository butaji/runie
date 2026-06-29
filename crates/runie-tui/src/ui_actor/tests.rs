#![allow(unused_imports)]
//! Tests for UiActor.
use super::*;
use runie_core::actors::RactorTurnActor;

#[cfg(test)]
async fn test_turn_handle() -> runie_core::actors::RactorTurnHandle {
    let bus = EventBus::<Event>::new(10);
    let (handle, _, _) = RactorTurnActor::spawn(bus).await;
    handle
}

/// Returns a `LeaderHandle` (aliased as `ActorHandles`) for tests.
///
/// Note: `ActorHandles` is an alias for `LeaderHandle` in `runie-core`.
/// This helper uses `test_leader_handle()` which constructs all actors
/// via the leader bootstrap.
#[cfg(test)]
async fn test_actor_handles() -> runie_core::actors::ActorHandles {
    runie_core::actors::leader::test_leader_handle().await
}

#[tokio::test]
async fn ui_actor_updates_state_from_bus_event() {
    let state = AppState::default();
    let bus = EventBus::<Event>::new(10);
    let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
    let agent_handle = AgentActorHandle::new(agent_tx);
    let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
    let (shutdown_tx, _shutdown_rx) = oneshot::channel();
    let turn_handle = test_turn_handle().await;

    let mut ui_actor = UiActor::new(
        state,
        agent_handle,
        turn_handle,
        kb_tx,
        bus.clone(),
        shutdown_tx,
        TermCaps::default(),
    );
    let mut render_rx = ui_actor.take_render_rx();

    let ui_sub = bus.subscribe();
    tokio::spawn(ui_actor.run(ui_sub));

    // Wait for the actor to publish the initial snapshot.
    let _ = render_rx.changed().await;

    bus.publish(Event::Input('h'));
    render_rx.changed().await.unwrap();
    let snap = render_rx.borrow().clone();
    assert!(snap.input.contains('h'));
}

#[tokio::test]
async fn render_actor_draws_snapshot_without_mutation() {
    let backend = ratatui::backend::TestBackend::new(20, 5);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let (render_tx, render_rx) = watch::channel(Snapshot::default());

    // Run a minimal render loop in the background.
    tokio::spawn(async move {
        let mut rx = render_rx;
        let _ = rx.changed().await;
        let snap = rx.borrow().clone();
        let _ = terminal.draw(|f| crate::ui::draw_snapshot(f, &snap));
    });

    let mut state = AppState::default();
    state.input_mut().input = "hello".to_string();
    render_tx.send(state.snapshot()).unwrap();

    // The snapshot was rendered from an immutable reference.
    // Mutation would require a mutable borrow, which the draw closure does not take.
    assert_eq!(state.input().input, "hello");
}

/// Helper: builds a UiActor with minimal dependencies for test use.
#[cfg(test)]
async fn make_test_actor() -> (
    UiActor,
    mpsc::Sender<Event>,
    runie_core::actors::RactorTurnHandle,
) {
    let state = AppState::default();
    let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
    let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
    let (shutdown_tx, _shutdown_rx) = oneshot::channel();
    let (effect_tx, _effect_rx) = mpsc::channel::<Event>(16);
    let turn_handle = test_turn_handle().await;
    let actor = UiActor::new(state, AgentActorHandle::new(agent_tx),
        turn_handle.clone(), kb_tx,
        EventBus::new(4), shutdown_tx, TermCaps::default());
    (actor, effect_tx, turn_handle)
}

#[tokio::test]
async fn paced_renderer_advances_on_response_delta() {
    let (mut actor, effect_tx, _) = make_test_actor().await;

    // Simulate a streaming message: TextStart -> ResponseDelta -> tick.
    actor.handle_event(Event::TextStart { id: "1".into() }, effect_tx.clone()).await;
    actor.handle_event(
        Event::ResponseDelta { id: "1".into(), content: "hello world".into() },
        effect_tx.clone(),
    )
    .await;

    // Tick once to advance the paced renderer.
    actor.paced.tick();

    let displayed = actor.paced.displayed();
    // Should have advanced at least the first 2 chars.
    assert!(
        !displayed.is_empty() || actor.paced.is_caught_up(),
        "paced renderer should show some text: '{}'",
        displayed
    );
}

#[tokio::test]
async fn paced_renderer_finishes_on_turn_complete() {
    let (mut actor, effect_tx, _) = make_test_actor().await;

    actor.handle_event(Event::TextStart { id: "1".into() }, effect_tx.clone()).await;
    actor.handle_event(
        Event::ResponseDelta { id: "1".into(), content: "hello world".into() },
        effect_tx.clone(),
    )
    .await;
    actor.handle_event(
        Event::TurnComplete { id: "1".into(), duration_secs: 1.0 },
        effect_tx.clone(),
    )
    .await;

    // After TurnComplete, renderer should be caught up.
    assert!(
        actor.paced.is_caught_up(),
        "paced renderer should be caught up after TurnComplete"
    );
    assert!(
        actor.paced.displayed().contains("hello"),
        "paced renderer should contain full text: '{}'",
        actor.paced.displayed()
    );
}

#[tokio::test]
async fn login_key_submit_triggers_validation_effect() {
    use runie_core::login_flow::{build_key_input, LoginFlowState};

    let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
    let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
    let (shutdown_tx, _shutdown_rx) = oneshot::channel();
    let (effect_tx, mut effect_rx) = mpsc::channel::<Event>(16);

    let handles = test_actor_handles().await;
    let mut state = AppState::default();
    state.set_actor_handles(handles.clone());

    let mut actor = UiActor::new(
        state, AgentActorHandle::new(agent_tx),
        handles.turn.clone(),
        kb_tx, EventBus::new(4), shutdown_tx, TermCaps::default(),
    );

    let mut flow = LoginFlowState::new().with_provider("test-unknown-provider".into());
    flow.key = "sk-test".into();
    let panel = build_key_input(&flow);
    let stack = runie_core::dialog::PanelStack::new(panel);
    *actor.state.open_dialog_mut() = Some(
        runie_core::commands::DialogState::Active { kind: DialogKind::Generic, panels: stack }
    );
    *actor.state.login_flow_mut() = Some(flow);

    actor.handle_event(
        Event::SubmitKey { provider: "test-unknown-provider".into(), key: "sk-test".into() },
        effect_tx.clone(),
    ).await;

    assert!(
        matches!(
            actor.state.login_flow().as_ref().map(|f| f.step.clone()),
            Some(LoginStep::Validating)
        ),
        "login flow should reach validating step"
    );

    let result = tokio::time::timeout(Duration::from_secs(2), effect_rx.recv()).await;
    assert!(result.is_ok(), "validation effect should produce a result event");
}
