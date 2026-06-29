//! `AgentActor` — ractor-based implementation.
//!
//! This is the production implementation of the AgentActor using ractor.

use std::pin::Pin;
use std::sync::Arc;
use parking_lot::Mutex;

use ractor::{Actor, ActorRef, ActorProcessingErr};
use ractor::async_trait;

use runie_core::actors::permission::RactorPermissionHandle;
use runie_core::actors::provider::RactorProviderHandle;
use runie_core::actors::ractor_adapter::{RactorHandle, spawn_ractor};
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::permissions::{
    DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove, PermissionManager,
};
use runie_core::permissions::PermissionGate;

use crate::emit_approval_sink::EmitApprovalSink;
use crate::run_agent_turn;
use crate::AgentCommand;

// ── Messages ───────────────────────────────────────────────────────────────────

/// Messages accepted by `AgentActor`.
#[derive(Clone, Debug)]
pub enum AgentMsg {
    /// Execute one agent turn.
    Run { command: AgentCommand },
    /// Execute a turn from the leader's minimal command type.
    RunLeader { command: runie_core::actors::leader::LeaderAgentCmd },
}

// ── Ractor-based AgentActor ───────────────────────────────────────────────────

/// Ractor-based AgentActor state.
struct RactorAgentActor {
    provider_handle: Arc<Mutex<Option<RactorProviderHandle>>>,
    permission_handle: Arc<Mutex<Option<RactorPermissionHandle>>>,
    bus: EventBus<Event>,
}

/// Spawn arguments for RactorAgentActor.
pub struct RactorAgentArgs {
    pub provider_handle: RactorProviderHandle,
    pub permission_handle: RactorPermissionHandle,
}

#[async_trait]
impl Actor for RactorAgentActor {
    type Msg = AgentMsg;
    type State = ();
    type Arguments = RactorAgentArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        *self.provider_handle.lock() = Some(args.provider_handle);
        *self.permission_handle.lock() = Some(args.permission_handle);
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            AgentMsg::Run { command } => self.run_turn(&command).await,
            AgentMsg::RunLeader { command } => {
                let cmd: AgentCommand = command.into();
                self.run_turn(&cmd).await;
            }
        }
        Ok(())
    }
}

impl RactorAgentActor {
    async fn run_turn(&self, command: &AgentCommand) {
        let provider = self.get_provider_handle(command);
        let permission = self.get_permission_handle(command);

        let (provider_key, model) = self.extract_provider_info(command);
        let built = match provider.build(provider_key, model).await {
            Ok(b) => b,
            Err(e) => {
                self.emit_error_and_done(&command.id, format!("Provider error: {e}"));
                return;
            }
        };

        let emit = self.create_emit_closure();
        let gate = self.create_permission_gate(permission);

        if let Err(e) = run_agent_turn(&built, command, emit, 5, gate).await {
            self.emit_error_and_done(&command.id, format!("Agent error: {e}"));
        }
    }

    fn get_provider_handle(&self, cmd: &AgentCommand) -> RactorProviderHandle {
        self.provider_handle
            .lock()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| {
                self.emit_error_and_done(&cmd.id, "Provider not initialized".into());
                panic!("Provider not initialized")
            })
    }

    fn get_permission_handle(&self, cmd: &AgentCommand) -> RactorPermissionHandle {
        self.permission_handle
            .lock()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| {
                self.emit_error_and_done(&cmd.id, "Permission handle not initialized".into());
                panic!("Permission handle not initialized")
            })
    }

    fn extract_provider_info(&self, command: &AgentCommand) -> (String, String) {
        if runie_core::provider::is_mock_enabled() {
            ("mock".to_owned(), "echo".to_owned())
        } else {
            (command.provider.clone(), command.model.clone())
        }
    }

    fn create_emit_closure(&self) -> Arc<Mutex<impl Fn(Event) + Send + Sync + 'static>> {
        let bus = self.bus.clone();
        Arc::new(Mutex::new(move |evt: Event| {
            bus.publish(evt);
        }))
    }

    fn create_permission_gate(&self, permission_handle: RactorPermissionHandle) -> PermissionGate {
        let permissions = PermissionManager::default().with_policies(vec![
            Box::new(DefaultToolApprove::new()),
            Box::new(GitTrackedWriteApprove::new()),
            Box::new(FileAccessAsk::new()),
        ]);
        PermissionGate::new(
            permissions,
            Arc::new(EmitApprovalSink::new(permission_handle)),
        )
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

/// Handle for the ractor-based AgentActor.
pub type RactorAgentHandle = RactorHandle<AgentMsg>;

/// Spawn a ractor-based AgentActor.
pub async fn spawn_ractor_agent(
    bus: runie_core::bus::EventBus<Event>,
    provider_handle: RactorProviderHandle,
    permission_handle: RactorPermissionHandle,
) -> Result<(RactorAgentHandle, ractor::concurrency::JoinHandle<()>, ractor::ActorCell), ractor::SpawnErr> {
    let actor = RactorAgentActor {
        provider_handle: Arc::new(Mutex::new(None)),
        permission_handle: Arc::new(Mutex::new(None)),
        bus: bus.clone(),
    };
    let args = RactorAgentArgs {
        provider_handle,
        permission_handle,
    };
    spawn_ractor(None, actor, args).await
}

/// Extension trait for RactorAgentHandle to add helper methods.
#[allow(async_fn_in_trait)]
pub trait RactorAgentHandleExt {
    /// Pop the next queued message and run a turn for it, if one is waiting.
    async fn run_if_queued(&self, turn_handle: &runie_core::actors::RactorTurnHandle);
}

impl RactorAgentHandleExt for RactorAgentHandle {
    async fn run_if_queued(&self, turn_handle: &runie_core::actors::RactorTurnHandle) {
        turn_handle.send(runie_core::actors::TurnMsg::RunIfQueued).await;
    }
}

// ── Leader integration ────────────────────────────────────────────────────────

impl From<runie_core::actors::leader::LeaderAgentCmd> for AgentCommand {
    fn from(cmd: runie_core::actors::leader::LeaderAgentCmd) -> Self {
        Self {
            content: cmd.content,
            id: cmd.id,
            provider: cmd.provider,
            model: cmd.model,
            thinking_level: cmd.thinking_level,
            read_only: cmd.read_only,
            skills_context: cmd.skills_context,
            system_prompt: cmd.system_prompt,
            truncation: crate::truncate::TruncationPolicy::default(),
        }
    }
}

/// Handle that implements `LeaderAgentHandle` for use by the leader.
pub struct LeaderAgentHandleImpl {
    inner: RactorAgentHandle,
}

impl LeaderAgentHandleImpl {
    pub fn new(inner: RactorAgentHandle) -> Self {
        Self { inner }
    }
}

impl runie_core::actors::leader::LeaderAgentHandle for LeaderAgentHandleImpl {
    fn run(&self, cmd: runie_core::actors::leader::LeaderAgentCmd) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let inner = self.inner.clone();
        let msg = AgentMsg::RunLeader { command: cmd };
        Box::pin(async move {
            let _ = inner.send(msg).await;
        })
    }
}

/// Factory for spawning agent actors (implements `AgentActorFactory`).
pub struct AgentActorFactoryImpl;

impl AgentActorFactoryImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentActorFactoryImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl runie_core::actors::leader::AgentActorFactory for AgentActorFactoryImpl {
    type SpawnFuture = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Box<dyn runie_core::actors::leader::LeaderAgentHandle>, ractor::SpawnErr>> + Send>>;

    fn spawn(
        &self,
        bus: runie_core::bus::EventBus<runie_core::event::Event>,
        provider_handle: runie_core::actors::provider::RactorProviderHandle,
        permission_handle: runie_core::actors::permission::RactorPermissionHandle,
    ) -> Self::SpawnFuture {
        Box::pin(async move {
            let (handle, _, _cell) =
                spawn_ractor_agent(bus, provider_handle, permission_handle).await?;
            Ok(Box::new(LeaderAgentHandleImpl::new(handle))
                as Box<dyn runie_core::actors::leader::LeaderAgentHandle>)
        })
    }
}
