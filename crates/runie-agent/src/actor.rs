//! `AgentActor` — owns interactive agent-turn execution.
//!
//! The actor receives `AgentMsg::Run` commands, builds the provider via the
//! shared `ProviderActor`, executes `run_agent_turn`, and publishes all progress
//! back to the `EventBus<Event>` so `UiActor` and `SessionActor` can react.

use std::sync::{Arc, Mutex};

use runie_core::actors::{spawn_actor, Actor, ActorHandle};
use runie_core::actors::ProviderActorHandle;
use runie_core::bus::EventBus;
use runie_core::event::{AgentEvent, Event};
use runie_core::AppState;
use runie_core::permissions::{
    ApprovalRegistry, DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove, PermissionManager,
};

use tokio::sync::mpsc;

use crate::emit_approval_sink::EmitApprovalSink;
use runie_core::permissions::PermissionGate;
use crate::run_agent_turn;
use crate::truncate::policy_from_section;
use crate::AgentCommand;

/// Messages accepted by `AgentActor`.
#[derive(Clone, Debug)]
pub enum AgentMsg {
    /// Execute one agent turn.
    Run { command: AgentCommand },
}

/// Ergonomic handle for sending commands to an `AgentActor`.
#[derive(Clone, Debug)]
pub struct AgentActorHandle {
    tx: mpsc::Sender<AgentMsg>,
}

impl AgentActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<AgentMsg>) -> Self {
        Self { tx }
    }

    /// Request that the actor run a turn.
    pub async fn run(&self, command: AgentCommand) {
        let _ = self.tx.send(AgentMsg::Run { command }).await;
    }

    /// Pop the next queued message and run a turn for it, if one is waiting.
    pub async fn run_if_queued(&self, state: &mut AppState) {
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

        self.run(AgentCommand {
            content,
            id,
            provider: state.config.current_provider.clone(),
            model: state.config.current_model.clone(),
            thinking_level: state.config.thinking_level,
            read_only: state.config.read_only,
            skills_context,
            system_prompt,
            truncation: policy_from_section(&state.config.truncation),
        })
        .await;
    }
}

/// Actor that executes agent turns and publishes progress to the event bus.
pub struct AgentActor {
    bus: EventBus<Event>,
    provider_handle: ProviderActorHandle,
    approval_registry: Arc<Mutex<ApprovalRegistry>>,
}

impl AgentActor {
    /// Spawn an `AgentActor` on the given event bus.
    pub fn spawn(
        bus: EventBus<Event>,
        provider_handle: ProviderActorHandle,
        approval_registry: Arc<Mutex<ApprovalRegistry>>,
    ) -> (AgentActorHandle, ActorHandle) {
        let actor_bus = bus.clone();
        let actor = Self {
            bus: actor_bus,
            provider_handle,
            approval_registry,
        };
        let (tx, handle) = spawn_actor(actor, bus);
        (AgentActorHandle::new(tx), handle)
    }
}

impl Actor for AgentActor {
    type Msg = AgentMsg;
    type Event = Event;

    async fn run_body(self, mut rx: mpsc::Receiver<Self::Msg>, _bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg).await;
        }
    }
}

impl AgentActor {
    async fn handle_msg(&self, msg: AgentMsg) {
        match msg {
            AgentMsg::Run { command } => self.run_turn(&command).await,
        }
    }

    async fn run_turn(&self, command: &AgentCommand) {
        let (provider_key, model) = if runie_core::provider::is_mock_enabled() {
            ("mock".to_string(), "echo".to_string())
        } else {
            (command.provider.clone(), command.model.clone())
        };
        let built = match self.provider_handle.build(provider_key, model).await {
            Ok(built) => built,
            Err(e) => {
                self.emit_error_and_done(&command.id, format!("Provider error: {e}"));
                return;
            }
        };

        // BuiltProvider implements Provider, so use it directly
        let emit = Arc::new(Mutex::new({
            let bus = self.bus.clone();
            move |evt: Event| {
                bus.publish(evt);
            }
        }));
        let permissions = PermissionManager::default().with_policies(vec![
            Box::new(DefaultToolApprove::new()),
            Box::new(GitTrackedWriteApprove::new()),
            Box::new(FileAccessAsk::new()),
        ]);
        let gate = PermissionGate::new(
            permissions,
            Arc::new(EmitApprovalSink::new(emit.clone(), self.approval_registry.clone())),
        );

        if let Err(e) = run_agent_turn(&built, command, emit, 5, gate).await {
            self.emit_error_and_done(&command.id, format!("Agent error: {e}"));
        }
    }

    fn emit_error_and_done(&self, id: &str, message: String) {
        self.bus.publish(AgentEvent::Error {
            id: id.to_string(),
            message,
        });
        self.bus.publish(AgentEvent::Done { id: id.to_string() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::actors::{ConfigActor, ProviderActor};
    use runie_provider::DynProviderFactory;

    fn test_command(provider: &str, model: &str) -> AgentCommand {
        AgentCommand {
            content: "hello".into(),
            id: "req.0".into(),
            provider: provider.into(),
            model: model.into(),
            thinking_level: runie_core::model::ThinkingLevel::Off,
            read_only: true,
            skills_context: String::new(),
            system_prompt: String::new(),
            truncation: crate::truncate::policy_from_section(&runie_core::config::TruncationSection {
                max_lines: 2000,
                max_bytes: 50_000,
            }),
        }
    }

    #[tokio::test]
    async fn actor_publishes_error_when_provider_unknown() {
        let _lock = crate::tests::MOCK_STATE_LOCK.lock().await;
        let was_mock = runie_core::provider::is_mock_enabled();
        runie_core::provider::set_mock_enabled(false);
        let bus = EventBus::<Event>::new(10);
        let mut sub = bus.subscribe();

        let (config_handle, _config_actor) = ConfigActor::spawn(bus.clone(), None);
        let (provider_handle, _provider_actor) =
            ProviderActor::spawn(bus.clone(), config_handle, Arc::new(DynProviderFactory));
        let registry = Arc::new(Mutex::new(ApprovalRegistry::default()));
        let (agent_handle, _agent_actor) = AgentActor::spawn(bus, provider_handle, registry);

        agent_handle.run(test_command("ghost-provider", "x")).await;

        let mut saw_error = false;
        let mut saw_done = false;
        for _ in 0..100 {
            if saw_error && saw_done {
                break;
            }
            tokio::task::yield_now().await;
            while let Ok(evt) = sub.try_recv() {
                match evt {
                    Event::Error { .. } => saw_error = true,
                    Event::Done { .. } => saw_done = true,
                    _ => {}
                }
            }
        }
        assert!(saw_error, "expected Error event for unknown provider");
        assert!(saw_done, "expected Done event after error");
        runie_core::provider::set_mock_enabled(was_mock);
    }

    #[tokio::test]
    async fn run_if_queued_sets_turn_active_and_inflight() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<AgentMsg>(10);
        let agent_handle = AgentActorHandle::new(tx);
        let mut state = AppState::default();
        state
            .agent
            .request_queue
            .push_back(("hello".to_string(), "req.0".to_string()));

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);

        agent_handle.run_if_queued(&mut state).await;

        assert!(state.agent.turn_active, "must set turn_active");
        assert_eq!(state.agent.inflight, 1, "must increment inflight");
        assert!(
            state.agent.request_queue.is_empty(),
            "message should be popped from queue"
        );

        let msg = rx.try_recv().expect("command should be sent to agent");
        let AgentMsg::Run { command } = msg;
        assert_eq!(command.content, "hello");
        assert_eq!(command.thinking_level, runie_core::model::ThinkingLevel::Off);
        assert_eq!(command.system_prompt, "");
    }

    #[tokio::test]
    async fn run_if_queued_noop_when_queue_empty() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<AgentMsg>(10);
        let agent_handle = AgentActorHandle::new(tx);
        let mut state = AppState::default();

        agent_handle.run_if_queued(&mut state).await;

        assert!(!state.agent.turn_active);
        assert_eq!(state.agent.inflight, 0);
    }

    #[tokio::test]
    async fn run_if_queued_noop_when_turn_active() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<AgentMsg>(10);
        let agent_handle = AgentActorHandle::new(tx);
        let mut state = AppState::default();
        state.agent.turn_active = true;
        state
            .agent
            .request_queue
            .push_back(("hello".to_string(), "req.0".to_string()));

        agent_handle.run_if_queued(&mut state).await;

        assert_eq!(state.agent.request_queue.len(), 1);
    }
}
