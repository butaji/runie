//! `AgentActor` — ractor-based implementation.
//!
//! This is the production implementation of the AgentActor using ractor.

use std::pin::Pin;
use std::sync::Arc;

use parking_lot::Mutex;
use ractor::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio_util::sync::CancellationToken;

use runie_core::actors::permission::RactorPermissionHandle;
use runie_core::actors::provider::RactorProviderHandle;
use runie_core::actors::ractor_adapter::spawn_ractor;
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::permissions::PermissionGate;
use runie_core::permissions::{
    DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove, PermissionManager,
    PermissionSetPolicy,
};

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
    RunLeader {
        command: runie_core::actors::leader::LeaderAgentCmd,
    },
}

// ── Ractor-based AgentActor ───────────────────────────────────────────────────

/// Ractor State for AgentActor — holds provider and permission handles.
struct AgentActorState {
    provider_handle: RactorProviderHandle,
    permission_handle: RactorPermissionHandle,
    bus: EventBus<Event>,
}

/// Unit struct for RactorAgentActor — state lives in `type State`.
struct RactorAgentActor;

/// Spawn arguments for RactorAgentActor.
pub struct RactorAgentArgs {
    pub provider_handle: RactorProviderHandle,
    pub permission_handle: RactorPermissionHandle,
    pub bus: EventBus<Event>,
}

#[async_trait]
impl Actor for RactorAgentActor {
    type Msg = AgentMsg;
    type State = AgentActorState;
    type Arguments = RactorAgentArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(AgentActorState {
            provider_handle: args.provider_handle,
            permission_handle: args.permission_handle,
            bus: args.bus,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            AgentMsg::Run { command } => self.run_turn(state, command).await,
            AgentMsg::RunLeader { command } => {
                let cmd: AgentCommand = command.into();
                self.run_turn(state, cmd).await;
            }
        }
        Ok(())
    }
}

impl RactorAgentActor {
    async fn run_turn(&self, state: &mut AgentActorState, mut command: AgentCommand) {
        let provider = state.provider_handle.clone();
        let permission = state.permission_handle.clone();
        let bus = state.bus.clone();

        let (provider_key, model) = if runie_core::provider::is_mock_enabled() {
            ("mock".to_owned(), "echo".to_owned())
        } else {
            (command.provider.clone(), command.model.clone())
        };

        let built = match provider.build(provider_key, model).await {
            Ok(b) => b,
            Err(e) => {
                Self::emit_error_and_done(state, &command.id, format!("Provider error: {e}"));
                return;
            }
        };

        let emit = Self::create_emit_closure(state);

        // Single cancellation token: cancelling it aborts both the provider stream
        // (via command.cancellation_token) and pending permission requests.
        let cancel_token = CancellationToken::new();
        command.cancellation_token = cancel_token.clone();
        let cancel_token_gate = cancel_token.clone();

        let gate =
            Self::create_permission_gate_with_cancel(permission.clone(), cancel_token_gate).await;
        let gate_for_abort = gate.clone();

        // Subscribe to TurnAborted so we can cancel pending permissions.
        let mut sub = bus.subscribe();

        let turn = run_agent_turn(&built, &command, emit, 5, gate.clone());
        tokio::pin!(turn);

        tokio::select! {
            result = &mut turn => {
                if let Err(e) = result {
                    Self::emit_error_and_done(state, &command.id, format!("Agent error: {e}"));
                }
            }
            // AbortTurn was received — cancel pending permissions and stop.
            _ = async {
                while let Ok(evt) = sub.recv().await {
                    if matches!(evt, Event::TurnAborted) {
                        break;
                    }
                }
            } => {
                // Cancel pending permission request via the cancellation token.
                cancel_token.cancel();
                gate_for_abort.cancel_pending();
            }
        }
    }

    fn create_emit_closure(
        state: &AgentActorState,
    ) -> Arc<Mutex<impl Fn(Event) + Send + Sync + 'static>> {
        let bus = state.bus.clone();
        Arc::new(Mutex::new(move |evt: Event| {
            bus.publish(evt);
        }))
    }

    async fn create_permission_gate_with_cancel(
        permission_handle: RactorPermissionHandle,
        abort_tx: tokio_util::sync::CancellationToken,
    ) -> PermissionGate {
        // Load user permission rules from PermissionActor. This includes rules
        // from [[permissions]] in config.toml and any /trust decisions.
        let rules = permission_handle.get_rules().await;

        let permissions = PermissionManager::default().with_policies(vec![
            Box::new(DefaultToolApprove::new()),
            Box::new(GitTrackedWriteApprove::new()),
            Box::new(FileAccessAsk::new()),
            // User declarative rules — added last so they take precedence
            // (PermissionSetPolicy.evaluate always returns Some, winning the chain).
            Box::new(PermissionSetPolicy::new(rules)),
        ]);
        PermissionGate::new(
            permissions,
            Arc::new(EmitApprovalSink::with_cancel(
                permission_handle,
                60,
                abort_tx,
            )),
        )
    }

    fn emit_error_and_done(state: &mut AgentActorState, id: &str, message: String) {
        state.bus.publish(runie_core::Event::Error {
            id: id.to_owned(),
            message,
        });
        state.bus.publish(runie_core::Event::Done { id: id.to_owned() });
    }
}

/// Handle for the ractor-based AgentActor.
pub type RactorAgentHandle = ractor::ActorRef<AgentMsg>;

/// Spawn a ractor-based AgentActor.
pub async fn spawn_ractor_agent(
    bus: runie_core::bus::EventBus<Event>,
    provider_handle: RactorProviderHandle,
    permission_handle: RactorPermissionHandle,
) -> Result<
    (
        RactorAgentHandle,
        ractor::concurrency::JoinHandle<()>,
        ractor::ActorCell,
    ),
    ractor::SpawnErr,
> {
    let args = RactorAgentArgs {
        provider_handle,
        permission_handle,
        bus: bus.clone(),
    };
    spawn_ractor(None, RactorAgentActor, args).await
}

/// Extension trait for RactorAgentHandle to add helper methods.
#[allow(async_fn_in_trait)]
pub trait RactorAgentHandleExt {
    /// Pop the next queued message and run a turn for it, if one is waiting.
    async fn run_if_queued(&self, turn_handle: &runie_core::actors::RactorTurnHandle);
}

impl RactorAgentHandleExt for RactorAgentHandle {
    async fn run_if_queued(&self, turn_handle: &runie_core::actors::RactorTurnHandle) {
        let _ = turn_handle.inner.send_message(runie_core::actors::TurnMsg::RunIfQueued);
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
            cancellation_token: CancellationToken::new(),
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
    fn run(
        &self,
        cmd: runie_core::actors::leader::LeaderAgentCmd,
    ) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let inner = self.inner.clone();
        let msg = AgentMsg::RunLeader { command: cmd };
        Box::pin(async move {
            let _ = inner.send_message(msg);
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
        Box<
            dyn std::future::Future<
                    Output = Result<
                        std::sync::Arc<dyn runie_core::actors::leader::LeaderAgentHandle>,
                        ractor::SpawnErr,
                    >,
                > + Send,
        >,
    >;

    fn spawn(
        &self,
        bus: runie_core::bus::EventBus<runie_core::event::Event>,
        provider_handle: runie_core::actors::provider::RactorProviderHandle,
        permission_handle: runie_core::actors::permission::RactorPermissionHandle,
    ) -> Self::SpawnFuture {
        Box::pin(async move {
            let (handle, _, _cell) =
                spawn_ractor_agent(bus, provider_handle, permission_handle).await?;
            Ok(std::sync::Arc::new(LeaderAgentHandleImpl::new(handle))
                as std::sync::Arc<dyn runie_core::actors::leader::LeaderAgentHandle>)
        })
    }

    fn spawn_with_join(
        &self,
        bus: runie_core::bus::EventBus<runie_core::event::Event>,
        provider_handle: runie_core::actors::provider::RactorProviderHandle,
        permission_handle: runie_core::actors::permission::RactorPermissionHandle,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<runie_core::actors::leader::SpawnedAgent, ractor::SpawnErr>> + Send>,
    > {
        Box::pin(async move {
            let (handle, join, _cell) =
                spawn_ractor_agent(bus, provider_handle, permission_handle).await?;
            Ok(runie_core::actors::leader::SpawnedAgent {
                handle: std::sync::Arc::new(LeaderAgentHandleImpl::new(handle))
                    as std::sync::Arc<dyn runie_core::actors::leader::LeaderAgentHandle>,
                join,
            })
        })
    }
}
