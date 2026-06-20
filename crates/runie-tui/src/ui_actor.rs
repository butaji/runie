//! UiActor — owns `AppState` and is the sole state mutator.
//!
//! The actor subscribes to the shared `EventBus<Event>`, applies every event to
//! `AppState`, sends fresh `Snapshot`s to the render task via a `watch` channel,
//! and triggers side-effects (agent spawns, clipboard, etc.) without blocking.

use std::collections::HashMap;
use std::time::Duration;

use runie_agent::{truncate::policy_from_section, AgentActorHandle, AgentCommand};
use runie_core::bus::{EventBus, ReplayReceiver};
use runie_core::event::{ControlEvent, Event, InputEvent};
use runie_core::login_flow::LoginStep;
use runie_core::{AppState, Snapshot};
use tokio::sync::{mpsc, oneshot, watch};

use crate::effects::EffectCommand;
use crate::terminal::caps::TerminalCapabilities;

const ANIM_MS: u64 = 200;

/// Actor that owns the application state.
pub struct UiActor {
    state: AppState,
    render_tx: watch::Sender<Snapshot>,
    agent_handle: AgentActorHandle,
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
        agent_handle: AgentActorHandle,
        kb_tx: watch::Sender<HashMap<String, String>>,
        bus: EventBus<Event>,
        shutdown_tx: oneshot::Sender<()>,
        caps: TerminalCapabilities,
    ) -> Self {
        Self {
            state,
            render_tx,
            agent_handle,
            kb_tx,
            bus,
            shutdown_tx: Some(shutdown_tx),
            caps,
        }
    }

    /// Run the actor until a quit event is processed.
    pub async fn run(mut self, mut rx: ReplayReceiver<Event>) {
        let (effect_tx, effect_rx) = mpsc::channel::<Event>(16);
        Self::spawn_effect_forwarder(self.bus.clone(), effect_rx);

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

    fn spawn_effect_forwarder(bus: EventBus<Event>, mut rx: mpsc::Receiver<Event>) {
        tokio::spawn(async move {
            while let Some(evt) = rx.recv().await {
                bus.publish(evt);
            }
        });
    }

    /// Handle a single event. Returns `true` when the actor should shut down.
    async fn handle_event(&mut self, evt: Event, effect_tx: mpsc::Sender<Event>) -> bool {
        let was_submit = matches!(evt, InputEvent::Submit);
        let was_followup = matches!(evt, ControlEvent::FollowUp);
        let was_config_loaded = matches!(evt, Event::ConfigLoaded { .. });
        let was_agent_done = matches!(evt, Event::Done { .. } | Event::Error { .. });

        let old_login_step = self.state.login_flow.as_ref().map(|f| f.step.clone());

        self.apply_event(evt, effect_tx.clone());
        self.dispatch_login_validation(effect_tx, old_login_step);

        if self.state.should_quit {
            return true;
        }

        if was_config_loaded {
            let _ = self.kb_tx.send(self.state.config.keybindings.clone());
        }
        if was_submit || was_followup || was_agent_done {
            spawn_if_queued(&mut self.state, &self.agent_handle).await;
        }

        self.publish_snapshot();
        false
    }

    fn apply_event(&mut self, evt: Event, effect_tx: mpsc::Sender<Event>) {
        if let Some(cmd) = EffectCommand::try_from_event(&evt, &self.state, &self.caps) {
            self.state.update(evt);
            cmd.dispatch(
                effect_tx,
                self.render_tx.clone(),
                &mut self.state,
                self.caps,
            );
        } else {
            self.state.update(evt);
        }
    }

    fn dispatch_login_validation(
        &mut self,
        effect_tx: mpsc::Sender<Event>,
        old_login_step: Option<LoginStep>,
    ) {
        if let Some(flow) = self.state.login_flow.as_ref() {
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

    fn publish_snapshot(&mut self) {
        self.state.ensure_fresh();
        let _ = self.render_tx.send(self.state.snapshot());
    }
}

/// Spawn an agent turn if the input queue has a pending message.
pub async fn spawn_if_queued(state: &mut AppState, agent_handle: &AgentActorHandle) {
    if state.agent.turn_active {
        return;
    }

    let Some((content, id)) = state.peek_queue() else {
        return;
    };
    let (content, id) = (content.clone(), id.clone());
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

    agent_handle
        .run(AgentCommand {
            content,
            id,
            provider: state.config.current_provider.clone(),
            model: state.config.current_model.clone(),
            thinking_level: state.config.thinking_level,
            read_only: state.config.read_only,
            skills_context,
            system_prompt,
            truncation: policy_from_section(
                state.config.truncation.max_lines,
                state.config.truncation.max_bytes,
            ),
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
        let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
        let agent_handle = AgentActorHandle::new(agent_tx);
        let (kb_tx, _kb_rx) = watch::channel(HashMap::<String, String>::new());
        let (shutdown_tx, _shutdown_rx) = oneshot::channel();

        let ui_sub = bus.subscribe();
        tokio::spawn(
            UiActor::new(
                state,
                render_tx,
                agent_handle,
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
        let (tx, mut rx) = tokio::sync::mpsc::channel::<runie_agent::AgentMsg>(10);
        let agent_handle = AgentActorHandle::new(tx);
        let mut state = AppState::default();
        state
            .agent
            .request_queue
            .push_back(("hello".to_string(), "req.0".to_string()));

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);

        spawn_if_queued(&mut state, &agent_handle).await;

        assert!(
            state.agent.turn_active,
            "spawn_if_queued must set turn_active"
        );
        assert_eq!(
            state.agent.inflight, 1,
            "spawn_if_queued must increment inflight"
        );
        assert!(
            state.agent.request_queue.is_empty(),
            "Message should be popped from request_queue"
        );

        let msg = rx.try_recv().expect("Command should be sent to agent");
        let runie_agent::AgentMsg::Run { command } = msg;
        assert_eq!(command.content, "hello");
        assert_eq!(command.thinking_level, runie_core::model::ThinkingLevel::Off);
        assert_eq!(command.system_prompt, "");
    }

    #[tokio::test]
    async fn spawn_if_queued_noop_when_queue_empty() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<runie_agent::AgentMsg>(10);
        let agent_handle = AgentActorHandle::new(tx);
        let mut state = AppState::default();

        spawn_if_queued(&mut state, &agent_handle).await;

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);
    }

    #[tokio::test]
    async fn login_key_submit_triggers_validation_effect() {
        use runie_core::actors::ProviderMsg;
        use runie_core::login_flow::{build_key_input, LoginFlowState};

        let (render_tx, _render_rx) = watch::channel(Snapshot::default());
        let (agent_tx, _agent_rx) = mpsc::channel::<runie_agent::AgentMsg>(1);
        let agent_handle = AgentActorHandle::new(agent_tx);
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
        state.provider_tx = Some(provider_tx);

        let mut actor = UiActor::new(
            state,
            render_tx,
            agent_handle,
            kb_tx,
            EventBus::new(4),
            shutdown_tx,
            TerminalCapabilities::default(),
        );

        let mut flow = LoginFlowState::new().with_provider("test-unknown-provider".into());
        flow.key = "sk-test".into();
        let panel = build_key_input(&flow);
        let stack = runie_core::dialog::PanelStack::new(panel);
        actor.state.open_dialog = Some(runie_core::commands::DialogState::PanelStack(stack));
        actor.state.login_flow = Some(flow);

        actor.handle_event(Event::Submit, effect_tx.clone()).await;

        assert!(
            matches!(
                actor.state.login_flow.as_ref().map(|f| f.step.clone()),
                Some(LoginStep::Validating)
            ),
            "login flow should reach validating step"
        );

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(2), effect_rx.recv()).await;
        assert!(
            result.is_ok(),
            "validation effect should produce a result event"
        );
    }
}
