//! `AgentActor` — owns interactive agent-turn execution.
//!
//! The actor receives `AgentMsg::Run` commands, builds the provider via the
//! shared `ProviderActor`, executes `run_agent_turn`, and publishes all progress
//! back to the `EventBus<Event>` so `UiActor` and `SessionActor` can react.

use std::sync::{Arc, Mutex};

use runie_core::actors::PermissionActorHandle;
use runie_core::actors::ProviderActorHandle;
use runie_core::actors::{spawn_actor, Actor, ActorHandle};
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::permissions::{
    DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove, PermissionManager,
};

use tokio::sync::mpsc;

use crate::emit_approval_sink::EmitApprovalSink;
use crate::run_agent_turn;
use crate::AgentCommand;
use runie_core::permissions::PermissionGate;

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
    ///
    /// This method sends `TurnMsg::RunIfQueued` to the TurnActor, which owns
    /// the authoritative queue state. The TurnActor will emit `TurnStarted`
    /// when it pops a message and starts a turn.
    pub async fn run_if_queued(&self, turn_handle: &runie_core::actors::RactorTurnHandle) {
        turn_handle.send(runie_core::actors::TurnMsg::RunIfQueued).await;
    }
}

/// Actor that executes agent turns and publishes progress to the event bus.
pub struct AgentActor {
    bus: EventBus<Event>,
    provider_handle: ProviderActorHandle,
    permission_handle: PermissionActorHandle,
}

impl AgentActor {
    /// Spawn an `AgentActor` on the given event bus.
    pub fn spawn(
        bus: EventBus<Event>,
        provider_handle: ProviderActorHandle,
        permission_handle: PermissionActorHandle,
    ) -> (AgentActorHandle, ActorHandle) {
        let actor_bus = bus.clone();
        let actor = Self {
            bus: actor_bus,
            provider_handle,
            permission_handle,
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
            ("mock".to_owned(), "echo".to_owned())
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
            Arc::new(EmitApprovalSink::new(self.permission_handle.clone())),
        );

        if let Err(e) = run_agent_turn(&built, command, emit, 5, gate).await {
            self.emit_error_and_done(&command.id, format!("Agent error: {e}"));
        }
    }

    fn emit_error_and_done(&self, id: &str, message: String) {
        self.bus.publish(runie_core::Event::Error {
            id: id.to_owned(),
            message,
        });
        self.bus
            .publish(runie_core::Event::Done { id: id.to_owned() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::actors::{ConfigActor, PermissionActor, ProviderActor};
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
            truncation: crate::truncate::policy_from_section(
                &runie_core::config::TruncationSection {
                    max_lines: 2000,
                    max_bytes: 50_000,
                },
            ),
        }
    }

    #[tokio::test]
    async fn actor_publishes_error_when_provider_unknown() {
        use runie_core::actors::permission::RactorPermissionActor;
        let _lock = crate::tests::MOCK_STATE_LOCK.lock().await;
        let was_mock = runie_core::provider::is_mock_enabled();
        runie_core::provider::set_mock_enabled(false);
        let bus = EventBus::<Event>::new(10);
        let mut sub = bus.subscribe();

        let (config_handle, _config_actor) = ConfigActor::spawn(bus.clone(), None);
        let (provider_handle, _provider_actor) =
            ProviderActor::spawn(bus.clone(), config_handle, Arc::new(DynProviderFactory));
        let (permission_handle, _permission_actor) = RactorPermissionActor::spawn(bus.clone()).await;
        let (agent_handle, _agent_actor) =
            AgentActor::spawn(bus, provider_handle, permission_handle);

        agent_handle.run(test_command("ghost-provider", "x")).await;

        // Wait for events to be processed - use recv with timeout
        let mut saw_error = false;
        let mut saw_done = false;
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
        
        while tokio::time::Instant::now() < deadline {
            if saw_error && saw_done {
                break;
            }
            // Try non-blocking receive first
            while let Ok(evt) = sub.try_recv() {
                match evt {
                    Event::Error { .. } => saw_error = true,
                    Event::Done { .. } => saw_done = true,
                    _ => {}
                }
            }
            if saw_error && saw_done {
                break;
            }
            // Then wait a bit
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        
        assert!(saw_error, "expected Error event for unknown provider");
        assert!(saw_done, "expected Done event after error");
        runie_core::provider::set_mock_enabled(was_mock);
    }

    #[tokio::test]
    async fn run_if_queued_sends_run_if_queued_to_turn_actor() {
        let bus = runie_core::bus::EventBus::<Event>::new(16);
        let (turn_handle, _, _) = runie_core::actors::RactorTurnActor::spawn(bus.clone()).await;
        let (tx, _rx) = tokio::sync::mpsc::channel::<AgentMsg>(10);
        let agent_handle = AgentActorHandle::new(tx);

        // Subscribe BEFORE operations so we can receive events
        let mut sub = bus.subscribe();

        // Queue a message via TurnActor
        turn_handle.send(runie_core::actors::TurnMsg::SubmitUserMessage {
            content: "hello".into(),
            id: "req.0".into(),
        }).await;

        // Call run_if_queued - this sends TurnMsg::RunIfQueued to TurnActor
        agent_handle.run_if_queued(&turn_handle).await;

        // Verify TurnStarted was emitted (TurnActor handles RunIfQueued and emits this)
        let mut found = false;
        let mut content = None;
        while let Ok(evt) = sub.recv().await {
            if let Event::TurnStarted { content: c, .. } = evt {
                found = true;
                content = Some(c);
                break;
            }
        }
        assert!(found, "TurnStarted should be emitted");
        assert_eq!(content.as_deref(), Some("hello"), "TurnStarted should contain queued content");
    }

    #[tokio::test]
    async fn run_if_queued_noop_when_turn_active() {
        let bus = runie_core::bus::EventBus::<Event>::new(16);
        let (turn_handle, _, _) = runie_core::actors::RactorTurnActor::spawn(bus.clone()).await;
        let (tx, _rx) = tokio::sync::mpsc::channel::<AgentMsg>(10);
        let agent_handle = AgentActorHandle::new(tx);

        // Subscribe BEFORE operations so we can receive events
        let mut sub = bus.subscribe();

        // Queue and start a turn
        turn_handle.send(runie_core::actors::TurnMsg::SubmitUserMessage {
            content: "hello".into(),
            id: "req.0".into(),
        }).await;
        turn_handle.send(runie_core::actors::TurnMsg::RunIfQueued).await;

        // Wait for first TurnStarted
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnStarted { .. }) {
                break;
            }
        }

        // Queue another message
        turn_handle.send(runie_core::actors::TurnMsg::SubmitUserMessage {
            content: "world".into(),
            id: "req.1".into(),
        }).await;

        // Try to run again (should be noop since turn is active)
        agent_handle.run_if_queued(&turn_handle).await;

        // Verify no additional TurnStarted was emitted (use try_recv with timeout)
        let mut turn_started_count = 1; // We already saw one
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(100);
        while tokio::time::Instant::now() < deadline {
            match sub.try_recv() {
                Ok(evt) => {
                    if matches!(evt, Event::TurnStarted { .. }) {
                        turn_started_count += 1;
                    }
                }
                Err(_) => break,
            }
        }
        assert_eq!(turn_started_count, 1, "only one TurnStarted should be emitted");
    }
}
