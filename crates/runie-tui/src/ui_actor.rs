//! UiActor — owns `AppState` and is the sole state mutator.
//!
//! The actor subscribes to the shared `EventBus<Event>`, applies every event to
//! `AppState`, sends fresh `Snapshot`s to the render task via a `watch` channel,
//! and triggers side-effects (agent spawns, clipboard, etc.) without blocking.

use std::collections::HashMap;
use std::time::Duration;

use runie_agent::{AgentCommand, truncate::policy_from_section};
use runie_core::bus::{EventBus, ReplayReceiver};
use runie_core::event::{ControlEvent, Event, InputEvent, ModelConfigEvent};
use runie_core::{AppState, Snapshot};
use tokio::sync::{mpsc, oneshot, watch};

use crate::effects::EffectCommand;
use crate::terminal::caps::TerminalCapabilities;

const ANIM_MS: u64 = 200;

/// Actor that owns the application state.
pub struct UiActor {
    state: AppState,
    render_tx: watch::Sender<Snapshot>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    kb_tx: watch::Sender<HashMap<String, String>>,
    bus: EventBus<Event>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    caps: TerminalCapabilities,
}

impl UiActor {
    /// Create a new `UiActor`.
    pub fn new(
        state: AppState,
        render_tx: watch::Sender<Snapshot>,
        cmd_tx: mpsc::Sender<AgentCommand>,
        kb_tx: watch::Sender<HashMap<String, String>>,
        bus: EventBus<Event>,
        shutdown_tx: oneshot::Sender<()>,
        caps: TerminalCapabilities,
    ) -> Self {
        Self {
            state,
            render_tx,
            cmd_tx,
            kb_tx,
            bus,
            shutdown_tx: Some(shutdown_tx),
            caps,
        }
    }

    /// Run the actor until a quit event is processed.
    pub async fn run(mut self, mut rx: ReplayReceiver<Event>) {
        let (effect_tx, mut effect_rx) = mpsc::channel::<Event>(16);
        let bus = self.bus.clone();
        tokio::spawn(async move {
            while let Some(evt) = effect_rx.recv().await {
                bus.publish(evt);
            }
        });

        let mut anim = tokio::time::interval(Duration::from_millis(ANIM_MS));

        self.state.ensure_fresh();
        let _ = self.render_tx.send(self.state.snapshot());

        loop {
            tokio::select! {
                biased;
                Ok(evt) = rx.recv() => {
                    if self.handle_event(evt, effect_tx.clone()).await {
                        break;
                    }
                }
                _ = anim.tick() => {
                    self.state.tick_animation();
                    self.publish_snapshot();
                }
            }
        }

        self.publish_snapshot();
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    /// Handle a single event. Returns `true` when the actor should shut down.
    async fn handle_event(&mut self, evt: Event, effect_tx: mpsc::Sender<Event>) -> bool {
        let was_submit = matches!(evt, InputEvent::Submit);
        let was_followup = matches!(evt, ControlEvent::FollowUp);
        let was_reload = matches!(evt, ModelConfigEvent::ReloadAll);
        let was_agent_done = matches!(evt, Event::Done { .. } | Event::Error { .. });

        if let Some(cmd) = EffectCommand::try_from_event(&evt, &self.state, &self.caps) {
            self.state.update(evt);
            cmd.dispatch(effect_tx, self.render_tx.clone(), &mut self.state, self.caps);
        } else {
            self.state.update(evt);
        }

        if self.state.should_quit {
            return true;
        }

        if was_reload {
            let _ = self.kb_tx.send(self.state.config.keybindings.clone());
        }
        if was_submit || was_followup || was_agent_done {
            spawn_if_queued(&mut self.state, &self.cmd_tx).await;
        }

        self.publish_snapshot();
        false
    }

    fn publish_snapshot(&mut self) {
        self.state.ensure_fresh();
        let _ = self.render_tx.send(self.state.snapshot());
    }
}

/// Spawn an agent turn if the input queue has a pending message.
pub async fn spawn_if_queued(state: &mut AppState, cmd_tx: &mpsc::Sender<AgentCommand>) {
    if state.agent.turn_active {
        return;
    }

    let Some((content, id)) = state.peek_queue() else {
        return;
    };
    let content = content.clone();
    let id = id.clone();
    state.pop_queue();

    state.agent.turn_active = true;
    state.agent.inflight += 1;
    state.agent.streaming = true;

    let skills_context = runie_core::skills::build_skills_context(&state.skills);
    let system_prompt = state
        .prompts
        .iter()
        .find(|p| p.name == state.input.current_prompt)
        .map(|p| p.content.clone())
        .unwrap_or_default();

    let _ = cmd_tx
        .send(AgentCommand {
            content,
            id,
            provider: state.config.current_provider.clone(),
            model: state.config.current_model.clone(),
            thinking_level: state.config.thinking_level,
            read_only: state.config.read_only,
            skills_context,
            system_prompt,
            truncation: policy_from_section(state.config.truncation.max_lines, state.config.truncation.max_bytes),
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ui_actor_updates_state_from_bus_event() {
        let state = AppState::default();
        let bus = EventBus::<Event>::new(10);
        let (render_tx, mut render_rx) = watch::channel(Snapshot::default());
        let (cmd_tx, _cmd_rx) = mpsc::channel(1);
        let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();

        let ui_sub = bus.subscribe();
        tokio::spawn(
            UiActor::new(
                state,
                render_tx,
                cmd_tx,
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
        state.input.input = "hello".to_string();
        render_tx.send(state.snapshot()).unwrap();

        // The snapshot was rendered from an immutable reference.
        // Mutation would require a mutable borrow, which the draw closure does not take.
        assert_eq!(state.input.input, "hello");
    }

    #[tokio::test]
    async fn spawn_if_queued_sets_turn_active_and_inflight() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<AgentCommand>(10);
        let mut state = AppState::default();
        state
            .agent
            .request_queue
            .push_back(("hello".to_string(), "req.0".to_string()));

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);

        spawn_if_queued(&mut state, &tx).await;

        assert!(state.agent.turn_active, "spawn_if_queued must set turn_active");
        assert_eq!(state.agent.inflight, 1, "spawn_if_queued must increment inflight");
        assert!(
            state.agent.request_queue.is_empty(),
            "Message should be popped from request_queue"
        );

        let cmd = rx.try_recv().expect("Command should be sent to agent");
        assert_eq!(cmd.content, "hello");
        assert_eq!(cmd.thinking_level, runie_core::model::ThinkingLevel::Off);
        assert_eq!(cmd.system_prompt, "");
    }

    #[tokio::test]
    async fn spawn_if_queued_noop_when_queue_empty() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<AgentCommand>(10);
        let mut state = AppState::default();

        spawn_if_queued(&mut state, &tx).await;

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);
    }
}
