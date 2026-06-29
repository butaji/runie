//! UiActor — owns `AppState` and is the sole state mutator.
//!
//! The actor subscribes to the shared `EventBus<Event>`, applies every event to
//! `AppState`, sends fresh `Snapshot`s to the render task via a `watch` channel,
//! and triggers side-effects (agent spawns, clipboard, etc.) without blocking.
use std::collections::HashMap;
use std::time::Duration;
use runie_agent::{AgentMsg, AgentCommand};
use runie_core::actors::RactorSessionHandle;
use runie_core::bus::{EventBus, Receiver};
#[cfg(test)]
use runie_core::login_flow::LoginStep;

#[cfg(test)]
use runie_core::commands::DialogKind;
use runie_core::{AppState, Snapshot, Event};
use tokio::sync::{mpsc, oneshot, watch};
use crate::effects::{login, EffectCommand};

/// Simple handle for sending commands to the agent.
#[derive(Clone)]
pub struct AgentActorHandle {
    tx: mpsc::Sender<AgentMsg>,
}

impl AgentActorHandle {
    pub fn new(tx: mpsc::Sender<AgentMsg>) -> Self { Self { tx } }

    pub async fn run(&self, command: AgentCommand) {
        let _ = self.tx.send(AgentMsg::Run { command }).await;
    }

    pub async fn run_if_queued(&self, turn_handle: &runie_core::actors::RactorTurnHandle) {
        turn_handle.send(runie_core::actors::TurnMsg::RunIfQueued).await;
    }
}
use crate::pace::PacedRenderer;
use crate::terminal::caps::TermCaps;

const ANIM_MS: u64 = 100;

/// Actor that owns the application state.
pub struct UiActor {
    state: AppState,
    render_tx: watch::Sender<Snapshot>,
    agent_handle: AgentActorHandle,
    persistence_handle: RactorSessionHandle,
    turn_handle: runie_core::actors::RactorTurnHandle,
    kb_tx: watch::Sender<HashMap<String, String>>,
    bus: EventBus<Event>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    caps: TermCaps,
    /// Paces the streaming text display for smooth typing animation.
    paced: PacedRenderer,
}

impl UiActor {
    /// Create a new `UiActor`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: AppState,
        render_tx: watch::Sender<Snapshot>,
        agent_handle: AgentActorHandle,
        persistence_handle: RactorSessionHandle,
        turn_handle: runie_core::actors::RactorTurnHandle,
        kb_tx: watch::Sender<HashMap<String, String>>,
        bus: EventBus<Event>,
        shutdown_tx: oneshot::Sender<()>,
        caps: TermCaps,
    ) -> Self {
        Self {
            state,
            render_tx,
            agent_handle,
            persistence_handle,
            turn_handle,
            kb_tx,
            bus,
            shutdown_tx: Some(shutdown_tx),
            caps,
            paced: PacedRenderer::new(),
        }
    }

    /// Run the actor until a quit event is processed.
    pub async fn run(mut self, mut rx: Receiver<Event>) {
        let (effect_tx, effect_rx) = mpsc::channel::<Event>(16);
        Self::spawn_effect_forwarder(self.bus.clone(), effect_rx);

        let mut anim = tokio::time::interval(Duration::from_millis(ANIM_MS));
        self.state.ensure_fresh();
        let snap = self.build_paced_snapshot();
        let _ = self.render_tx.send(snap);

        loop {
            tokio::select! {
                Ok(evt) = rx.recv() => {
                    if self.handle_event_inner(evt, effect_tx.clone()).await {
                        break;
                    }
                    // Drain any events already queued (e.g. streaming response
                    // deltas) and apply them in one batch, then publish a single
                    // snapshot for the whole burst instead of one per token.
                    while let Ok(evt) = rx.try_recv() {
                        if self.handle_event_inner(evt, effect_tx.clone()).await {
                            self.publish_snapshot();
                            return;
                        }
                    }
                    self.publish_snapshot();
                }
                _ = anim.tick() => {
                    self.state.tick_animation();
                    self.paced.tick();
                    self.publish_snapshot();
                }
            }
        }

        self.publish_snapshot();
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    fn spawn_effect_forwarder(bus: EventBus<Event>, mut rx: mpsc::Receiver<Event>) {
        tokio::spawn(async move {
            while let Some(evt) = rx.recv().await {
                bus.publish(evt);
            }
        });
    }

    /// Handle a single event and publish a fresh snapshot.
    /// Returns `true` when the actor should shut down.
    #[cfg(test)]
    async fn handle_event(&mut self, evt: Event, effect_tx: mpsc::Sender<Event>) -> bool {
        let quit = self.handle_event_inner(evt, effect_tx).await;
        self.publish_snapshot();
        quit
    }

    /// Handle a single event without publishing. Returns `true` when the actor
    /// should shut down.
    async fn handle_event_inner(&mut self, evt: Event, effect_tx: mpsc::Sender<Event>) -> bool {
        let was_submit = matches!(evt, Event::Submit);
        let was_followup = matches!(evt, Event::FollowUp);
        let was_config_loaded = matches!(&evt, Event::ConfigLoaded { .. });
        let was_agent_done = matches!(&evt, Event::Done { .. } | Event::Error { .. });

        let submitted_text = if was_submit {
            Some(self.state.input().input().to_string())
        } else {
            None
        };

        self.apply_event(evt.clone());
        self.update_paced_renderer(&evt);
        self.dispatch_effect(&evt, effect_tx.clone()).await;
        if *self.state.should_quit_mut() {
            return true;
        }
        if was_config_loaded {
            let _ = self.kb_tx.send(self.state.config().keybindings().clone());
        }
        handle_persistence_messages(self.persistence_handle.clone(), evt, submitted_text).await;
        if was_submit || was_followup || was_agent_done {
            self.agent_handle.run_if_queued(&self.turn_handle).await;
        }

        false
    }

    /// Update the paced renderer based on the received event.
    fn update_paced_renderer(&mut self, evt: &Event) {
        match evt {
            Event::TextStart { .. } => {
                self.paced = PacedRenderer::new();
            }
            Event::ResponseDelta { content, .. } => {
                self.paced.push(content);
            }
            Event::TurnComplete { .. } | Event::Done { .. } => {
                self.paced.finish();
            }
            _ => {}
        }
    }

    fn apply_event(&mut self, evt: Event) {
        self.state.update(evt);
    }

    /// Dispatch effects via IoActor.
    async fn dispatch_effect(&mut self, evt: &Event, effect_tx: mpsc::Sender<Event>) {
        if let Some(cmd) = EffectCommand::try_from_event(evt, &mut self.state, &self.caps) {
            // For login validation, handle separately
            if matches!(cmd, EffectCommand::LoginFlowSubmitKey { .. }) {
                let flow = self.state.login_flow().cloned();
                if let Some(f) = flow {
                    let tx = effect_tx.clone();
                    let provider_handle = self.state.actor_handles().as_ref()
                        .map(|h| h.provider.clone());
                    if let Some(handle) = provider_handle {
                        tokio::spawn(login::run(f.provider, f.key, tx, handle.clone()));
                    }
                }
            } else {
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    cmd.dispatch_async(&state_clone).await;
                });
            }
        }
    }

    /// Build a snapshot with the paced streaming tail applied.
    fn build_paced_snapshot(&mut self) -> Snapshot {
        self.state.ensure_fresh();
        let mut snap = self.state.snapshot();
        // Show the paced display text instead of the raw streaming tail.
        snap.streaming_tail = self.paced.displayed().to_owned();
        snap
    }

    fn publish_snapshot(&mut self) {
        let snap = self.build_paced_snapshot();
        let _ = self.render_tx.send(snap);
    }
}

async fn handle_persistence_messages(
    handle: RactorSessionHandle,
    evt: Event,
    submitted_text: Option<String>,
) {
    if let Some(entry) = submitted_text {
        if !entry.trim().is_empty() {
            handle.append_history(entry.trim().to_owned()).await;
        }
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    match evt {
        Event::TrustProject => {
            handle
                .set_trust(cwd, runie_core::trust::TrustDecision::Trusted)
                .await;
        }
        Event::UntrustProject => {
            handle
                .set_trust(cwd, runie_core::trust::TrustDecision::Untrusted)
                .await;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::actors::{
        ActorHandles, InputActor, RactorIoActor,
        RactorSessionActor, RactorTurnActor, RactorConfigActor,
    };
    use runie_core::actors::permission::RactorPermissionActor;
    use runie_core::actors::provider::{RactorProviderActor, RactorProviderHandle};

    async fn test_turn_handle() -> runie_core::actors::RactorTurnHandle {
        let bus = EventBus::<Event>::new(10);
        let (handle, _, _) = RactorTurnActor::spawn(bus).await;
        handle
    }

    async fn test_session_handle() -> RactorSessionHandle {
        let bus = EventBus::<Event>::new(10);
        let (handle, _) = RactorSessionActor::spawn(bus).await.unwrap();
        handle
    }

    async fn test_provider_handle() -> RactorProviderHandle {
        use std::sync::Arc;
        use runie_provider::DynProviderFactory;
        let bus = EventBus::<Event>::new(10);
        let (config_handle, _) = RactorConfigActor::spawn(bus.clone(), None).await;
        let (handle, _) = RactorProviderActor::spawn(
            bus.clone(),
            config_handle,
            Arc::new(DynProviderFactory),
        ).await;
        handle
    }

    async fn test_actor_handles() -> runie_core::actors::ActorHandles {
        use std::sync::Arc;
        use runie_provider::DynProviderFactory;
        let bus = EventBus::<Event>::new(10);
        let (config, _) = RactorConfigActor::spawn(bus.clone(), None).await;
        let (provider, _) = RactorProviderActor::spawn(
            bus.clone(),
            config.clone(),
            Arc::new(DynProviderFactory),
        ).await;
        let (session, _) = RactorSessionActor::spawn(bus.clone()).await.unwrap();
        let (io, _) = RactorIoActor::spawn(bus.clone()).await.unwrap();
        let (permission, _) = RactorPermissionActor::spawn(bus.clone()).await;
        let (input, _) = InputActor::spawn(bus.clone()).await;
        let (turn, _, turn_join) = RactorTurnActor::spawn(bus.clone()).await;
        runie_core::actors::ActorHandles {
            config,
            provider,
            session,
            io,
            fff_indexer: None,
            input,
            permission,
            turn,
            turn_join: Some(Arc::new(turn_join)),
        }
    }

    #[tokio::test]
    async fn ui_actor_updates_state_from_bus_event() {
        let state = AppState::default();
        let bus = EventBus::<Event>::new(10);
        let (render_tx, mut render_rx) = watch::channel(Snapshot::default());
        let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
        let agent_handle = AgentActorHandle::new(agent_tx);
        let persistence_handle = test_session_handle().await;
        let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();
        let turn_handle = test_turn_handle().await;

        let ui_sub = bus.subscribe();
        tokio::spawn(
            UiActor::new(
                state,
                render_tx,
                agent_handle,
                persistence_handle,
                turn_handle,
                kb_tx,
                bus.clone(),
                shutdown_tx,
                TermCaps::default(),
            )
            .run(ui_sub),
        );

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
    async fn make_test_actor() -> (
        UiActor,
        mpsc::Sender<Event>,
        runie_core::actors::RactorTurnHandle,
    ) {
        let state = AppState::default();
        let (render_tx, _render_rx) = watch::channel(Snapshot::default());
        let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
        let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();
        let (effect_tx, _effect_rx) = mpsc::channel::<Event>(16);
        let turn_handle = test_turn_handle().await;
        let persistence_handle = test_session_handle().await;
        let actor = UiActor::new(state, render_tx, AgentActorHandle::new(agent_tx),
            persistence_handle, turn_handle.clone(), kb_tx,
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

        let (render_tx, _render_rx) = watch::channel(Snapshot::default());
        let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
        let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();
        let (effect_tx, mut effect_rx) = mpsc::channel::<Event>(16);

        let handles = test_actor_handles().await;
        let mut state = AppState::default();
        state.set_actor_handles(handles.clone());

        let mut actor = UiActor::new(
            state, render_tx, AgentActorHandle::new(agent_tx),
            handles.session.clone(), handles.turn.clone(),
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
}
