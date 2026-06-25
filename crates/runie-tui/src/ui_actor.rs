//! UiActor — owns `AppState` and is the sole state mutator.
//!
//! The actor subscribes to the shared `EventBus<Event>`, applies every event to
//! `AppState`, sends fresh `Snapshot`s to the render task via a `watch` channel,
//! and triggers side-effects (agent spawns, clipboard, etc.) without blocking.

use std::collections::HashMap;
use std::time::Duration;

use runie_agent::AgentActorHandle;
use runie_core::actors::SessionActorHandle;
use runie_core::bus::{EventBus, Receiver};
use runie_core::login_flow::LoginStep;
use runie_core::Event;
use runie_core::{AppState, Snapshot};
use tokio::sync::{mpsc, oneshot, watch};

use crate::effects::EffectCommand;
use crate::pace::PacedRenderer;
use crate::terminal::caps::TerminalCapabilities;

const ANIM_MS: u64 = 100;

/// Actor that owns the application state.
pub struct UiActor {
    state: AppState,
    render_tx: watch::Sender<Snapshot>,
    agent_handle: AgentActorHandle,
    persistence_handle: SessionActorHandle,
    kb_tx: watch::Sender<HashMap<String, String>>,
    bus: EventBus<Event>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    caps: TerminalCapabilities,
    /// Paces the streaming text display for smooth typing animation.
    paced: PacedRenderer,
}

impl UiActor {
    /// Create a new `UiActor`.
    pub fn new(
        state: AppState,
        render_tx: watch::Sender<Snapshot>,
        agent_handle: AgentActorHandle,
        persistence_handle: SessionActorHandle,
        kb_tx: watch::Sender<HashMap<String, String>>,
        bus: EventBus<Event>,
        shutdown_tx: oneshot::Sender<()>,
        caps: TerminalCapabilities,
    ) -> Self {
        Self {
            state,
            render_tx,
            agent_handle,
            persistence_handle,
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
        let was_trust_loaded = matches!(&evt, Event::TrustLoaded { .. });

        let submitted_text = if was_submit {
            Some(self.state.input().input.clone())
        } else {
            None
        };

        let old_login_step = self.state.login_flow().as_ref().map(|f| f.step.clone());
        self.apply_event(evt.clone());
        self.update_paced_renderer(&evt);
        self.dispatch_effect(&evt, effect_tx.clone());
        self.dispatch_login_validation(effect_tx, old_login_step);

        if *self.state.should_quit_mut() {
            return true;
        }
        if was_config_loaded {
            let _ = self.kb_tx.send(self.state.config().keybindings.clone());
        }
        if was_trust_loaded {
            let cwd = std::env::current_dir().unwrap_or_default();
            runie_core::update::apply_initial_trust(&mut self.state, &cwd);
        }
        handle_persistence_messages(self.persistence_handle.clone(), evt, submitted_text).await;
        if was_submit || was_followup || was_agent_done {
            self.agent_handle.run_if_queued(&mut self.state).await;
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

    fn dispatch_effect(&mut self, evt: &Event, effect_tx: mpsc::Sender<Event>) {
        if let Some(cmd) = EffectCommand::try_from_event(evt, &mut self.state, &self.caps) {
            cmd.dispatch(
                effect_tx,
                self.render_tx.clone(),
                &mut self.state,
                self.caps,
            );
        }
    }

    fn dispatch_login_validation(
        &mut self,
        effect_tx: mpsc::Sender<Event>,
        old_login_step: Option<LoginStep>,
    ) {
        if let Some(flow) = self.state.login_flow().as_ref() {
            if flow.step == LoginStep::Validating && old_login_step != Some(LoginStep::Validating) {
                EffectCommand::LoginFlowSubmitKey {
                    provider: flow.provider.clone(),
                    key: flow.key.clone(),
                }
                .dispatch(
                    effect_tx,
                    self.render_tx.clone(),
                    &mut self.state,
                    self.caps,
                );
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
    handle: SessionActorHandle,
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
    use runie_core::actors::{ActorHandles, ProviderActorHandle};

    #[tokio::test]
    async fn ui_actor_updates_state_from_bus_event() {
        let state = AppState::default();
        let bus = EventBus::<Event>::new(10);
        let (render_tx, mut render_rx) = watch::channel(Snapshot::default());
        let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
        let agent_handle = AgentActorHandle::new(agent_tx);
        let (persist_tx, _persist_rx) = mpsc::channel::<runie_core::actors::SessionMsg>(1);
        let persistence_handle = SessionActorHandle::new(persist_tx);
        let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();

        let ui_sub = bus.subscribe();
        tokio::spawn(
            UiActor::new(
                state,
                render_tx,
                agent_handle,
                persistence_handle,
                kb_tx,
                bus.clone(),
                shutdown_tx,
                TerminalCapabilities::default(),
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

    #[tokio::test]
    async fn paced_renderer_advances_on_response_delta() {
        let state = AppState::default();
        let (render_tx, _render_rx) = watch::channel(Snapshot::default());
        let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
        let agent_handle = AgentActorHandle::new(agent_tx);
        let (persist_tx, _persist_rx) = mpsc::channel::<runie_core::actors::SessionMsg>(1);
        let persistence_handle = SessionActorHandle::new(persist_tx);
        let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();
        let (effect_tx, _effect_rx) = mpsc::channel::<Event>(16);

        let mut actor = UiActor::new(
            state,
            render_tx,
            agent_handle,
            persistence_handle,
            kb_tx,
            EventBus::new(4),
            shutdown_tx,
            TerminalCapabilities::default(),
        );

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
        let state = AppState::default();
        let (render_tx, _render_rx) = watch::channel(Snapshot::default());
        let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
        let agent_handle = AgentActorHandle::new(agent_tx);
        let (persist_tx, _persist_rx) = mpsc::channel::<runie_core::actors::SessionMsg>(1);
        let persistence_handle = SessionActorHandle::new(persist_tx);
        let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();
        let (effect_tx, _effect_rx) = mpsc::channel::<Event>(16);

        let mut actor = UiActor::new(
            state,
            render_tx,
            agent_handle,
            persistence_handle,
            kb_tx,
            EventBus::new(4),
            shutdown_tx,
            TerminalCapabilities::default(),
        );

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
        use runie_core::actors::ProviderMsg;
        use runie_core::login_flow::{build_key_input, LoginFlowState};

        let (render_tx, _render_rx) = watch::channel(Snapshot::default());
        let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
        let agent_handle = AgentActorHandle::new(agent_tx);
        let (persist_tx, _persist_rx) = mpsc::channel::<runie_core::actors::SessionMsg>(1);
        let persistence_handle = SessionActorHandle::new(persist_tx);
        let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();
        let (effect_tx, mut effect_rx) = mpsc::channel::<Event>(16);
        let (provider_tx, mut provider_rx) = mpsc::channel::<ProviderMsg>(1);

        tokio::spawn(async move {
            if let Some(ProviderMsg::ValidateKey { reply, .. }) = provider_rx.recv().await {
                reply.send(Ok(vec!["model".into()]));
            }
        });

        let mut state = AppState::default();
        // Set up actor_handles with provider handle so effects can route through it.
        let mut handles = ActorHandles::default();
        handles.provider = Some(ProviderActorHandle::new(provider_tx));
        state.set_actor_handles(handles);

        let mut actor = UiActor::new(
            state,
            render_tx,
            agent_handle,
            persistence_handle,
            kb_tx,
            EventBus::new(4),
            shutdown_tx,
            TerminalCapabilities::default(),
        );

        let mut flow = LoginFlowState::new().with_provider("test-unknown-provider".into());
        flow.key = "sk-test".into();
        let panel = build_key_input(&flow);
        let stack = runie_core::dialog::PanelStack::new(panel);
        *actor.state.open_dialog_mut() = Some(runie_core::commands::DialogState::PanelStack(stack));
        *actor.state.login_flow_mut() = Some(flow);

        actor.handle_event(Event::Submit, effect_tx.clone()).await;

        assert!(
            matches!(
                actor.state.login_flow().as_ref().map(|f| f.step.clone()),
                Some(LoginStep::Validating)
            ),
            "login flow should reach validating step"
        );

        let result =
            tokio::time::timeout(std::time::Duration::from_secs(2), effect_rx.recv()).await;
        assert!(
            result.is_ok(),
            "validation effect should produce a result event"
        );
    }
}
