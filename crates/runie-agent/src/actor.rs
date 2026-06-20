//! `AgentActor` — owns interactive agent-turn execution.
//!
//! The actor receives `AgentMsg::Run` commands, builds the provider via the
//! shared `ProviderActor`, executes `run_agent_turn`, and publishes all progress
//! back to the `EventBus<Event>` so `UiActor` and `SessionActor` can react.

use std::sync::{Arc, Mutex};

use runie_core::actor::{spawn_actor, Actor, ActorHandle};
use runie_core::actors::ProviderActorHandle;
use runie_core::bus::EventBus;
use runie_core::event::{AgentEvent, Event};
use runie_core::permissions::{
    ApprovalRegistry, DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove, PermissionManager,
};
use runie_provider::DynProvider;
use tokio::sync::mpsc;

use crate::emit_approval_sink::EmitApprovalSink;
use crate::permission_gate::PermissionGate;
use crate::run_agent_turn;
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
        let built = match self.provider_handle.build(command.provider.clone(), command.model.clone()).await {
            Ok(built) => built,
            Err(e) => {
                self.emit_error_and_done(&command.id, format!("Provider error: {e}"));
                return;
            }
        };

        let provider = DynProvider::from_provider(built.provider, &built.key, &built.model);
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

        if let Err(e) = run_agent_turn(&provider, command, emit, 5, gate).await {
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
            truncation: crate::truncate::policy_from_section(2000, 50_000),
        }
    }

    #[tokio::test]
    async fn actor_publishes_error_when_provider_unknown() {
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
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            while let Some(Ok(evt)) = sub.try_recv() {
                match evt {
                    Event::Error { .. } => saw_error = true,
                    Event::Done { .. } => saw_done = true,
                    _ => {}
                }
            }
        }
        assert!(saw_error, "expected Error event for unknown provider");
        assert!(saw_done, "expected Done event after error");
    }
}
