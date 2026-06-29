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

/// Handle backed by `LeaderAgentHandle` (from `Leader::start`).
/// Used when the agent is spawned via the leader bootstrap.
#[derive(Clone)]
pub struct LeaderAgentActorHandle {
    inner: std::sync::Arc<dyn runie_core::actors::leader::LeaderAgentHandle>,
}

impl LeaderAgentActorHandle {
    /// Wrap a `LeaderAgentHandle` (e.g. from `LeaderHandle::agent`).
    pub fn new(inner: std::sync::Arc<dyn runie_core::actors::leader::LeaderAgentHandle>) -> Self {
        Self { inner }
    }

    pub async fn run(&self, command: AgentCommand) {
        let cmd = runie_core::actors::leader::LeaderAgentCmd {
            content: command.content,
            id: command.id,
            provider: command.provider,
            model: command.model,
            thinking_level: command.thinking_level,
            read_only: command.read_only,
            skills_context: command.skills_context,
            system_prompt: command.system_prompt,
        };
        self.inner.run(cmd).await;
    }

    pub async fn run_if_queued(&self, turn_handle: &runie_core::actors::RactorTurnHandle) {
        turn_handle.send(runie_core::actors::TurnMsg::RunIfQueued).await;
    }
}
use crate::pace::PacedRenderer;
use crate::terminal::caps::TermCaps;

const ANIM_MS: u64 = 100;

/// Box over agent-handle variants so UiActor can hold either type without
/// generics or async-fn trait objects.
pub enum AgentHandleBox {
    Actor(AgentActorHandle),
    Leader(LeaderAgentActorHandle),
}

impl AgentHandleBox {
    async fn run(&self, command: AgentCommand) {
        match self {
            Self::Actor(h) => h.run(command).await,
            Self::Leader(h) => h.run(command).await,
        }
    }

    async fn run_if_queued(&self, turn: &runie_core::actors::RactorTurnHandle) {
        match self {
            Self::Actor(h) => h.run_if_queued(turn).await,
            Self::Leader(h) => h.run_if_queued(turn).await,
        }
    }
}

/// Actor that owns the application state.
pub struct UiActor {
    pub(crate) state: AppState,
    /// UiActor creates its own watch channel for snapshots so the render task can
    /// receive frames. Call `take_render_rx()` after construction to hand the
    /// receiver to the render task.
    render_tx: watch::Sender<Snapshot>,
    render_rx: Option<watch::Receiver<Snapshot>>,
    agent_handle: AgentHandleBox,
    persistence_handle: RactorSessionHandle,
    turn_handle: runie_core::actors::RactorTurnHandle,
    kb_tx: watch::Sender<HashMap<String, String>>,
    bus: EventBus<Event>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    caps: TermCaps,
    pub(crate) paced: PacedRenderer,
}

impl UiActor {
    /// Create a new `UiActor` with an mpsc-backed agent handle.
    /// UiActor creates its own watch channel for snapshots; call `take_render_rx()`
    /// to hand the receiver to the render task.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: AppState,
        agent_handle: AgentActorHandle,
        persistence_handle: RactorSessionHandle,
        turn_handle: runie_core::actors::RactorTurnHandle,
        kb_tx: watch::Sender<HashMap<String, String>>,
        bus: EventBus<Event>,
        shutdown_tx: oneshot::Sender<()>,
        caps: TermCaps,
    ) -> Self {
        Self::with_agent_handle(
            state, AgentHandleBox::Actor(agent_handle), persistence_handle,
            turn_handle, kb_tx, bus, shutdown_tx, caps,
        )
    }

    /// Create a new `UiActor` with a generic agent handle.
    /// UiActor creates its own watch channel for snapshots; call `take_render_rx()`
    /// to hand the receiver to the render task.
    #[allow(clippy::too_many_arguments)]
    pub fn with_agent_handle(
        mut state: AppState,
        agent_handle: AgentHandleBox,
        persistence_handle: RactorSessionHandle,
        turn_handle: runie_core::actors::RactorTurnHandle,
        kb_tx: watch::Sender<HashMap<String, String>>,
        bus: EventBus<Event>,
        shutdown_tx: oneshot::Sender<()>,
        caps: TermCaps,
    ) -> Self {
        let (render_tx, render_rx) = watch::channel(state.snapshot());
        Self {
            state,
            render_tx,
            render_rx: Some(render_rx),
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

    /// Take the snapshot channel receiver, transferring ownership to the render task.
    /// Must be called exactly once, after construction and before `run()`.
    pub fn take_render_rx(&mut self) -> watch::Receiver<Snapshot> {
        self.render_rx.take()
            .expect("render_rx already taken")
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

mod tests;
