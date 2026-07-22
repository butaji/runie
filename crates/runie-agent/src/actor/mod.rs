#![allow(clippy::too_many_lines)]

//! `AgentActor` — ractor-based implementation.
//!
//! This is the production implementation of the AgentActor using ractor.
//!
//! ## Module structure
//!
//! The implementation is split into focused submodules:
//! - `handlers.rs` — Turn setup, permission gate creation, turn spawning
//! - `leader.rs` — Leader integration and factory
//! - `tests.rs` — Unit tests

pub mod handlers;
pub mod leader;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use ractor::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio_util::sync::CancellationToken;

use runie_core::actors::permission::RactorPermissionHandle;
use runie_core::actors::provider::{BuiltProvider, RactorProviderHandle};
use runie_core::actors::ractor_adapter::spawn_ractor;
use runie_core::bus::EventBus;
use runie_core::event::Event;
use runie_core::permissions::PermissionGate;

use crate::constants::DEFAULT_MAX_TOOL_ROUNDS;
use crate::run_agent_turn;
use crate::stream_response::EmitFn;
use crate::AgentCommand;

// ── Messages ───────────────────────────────────────────────────────────────────

/// Messages accepted by `AgentActor`.
#[derive(Clone, Debug)]
pub enum AgentMsg {
    /// Execute one agent turn.
    Run { command: AgentCommand },
    /// Execute a turn from the leader's minimal command type.
    RunLeader { command: runie_core::actors::leader::LeaderAgentCmd },
    /// Abort the currently running turn, if any.
    Abort,
    /// Internal: the spawned turn task finished (success or error).
    /// Clears the in-flight turn state so a subsequent Run is accepted.
    TurnComplete,
}

// ── Ractor-based AgentActor ───────────────────────────────────────────────────

/// Ractor State for AgentActor — holds provider and permission handles.
pub(crate) struct AgentActorState {
    provider_handle: RactorProviderHandle,
    permission_handle: RactorPermissionHandle,
    bus: EventBus<Event>,
    /// Cancellation token for the currently running turn, if any.
    /// Stored in Arc so it can be taken on Abort.
    current_turn_token: Option<Arc<CancellationToken>>,
    /// Permission gate for the currently running turn, if any.
    current_gate: Option<PermissionGate>,
    /// JoinHandle for the currently running turn task, if any.
    /// Used to await completion and capture errors.
    current_turn_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Unit struct for RactorAgentActor — state lives in `type State`.
struct RactorAgentActor;

/// Spawn arguments for RactorAgentActor.
pub struct RactorAgentArgs {
    pub provider_handle: RactorProviderHandle,
    pub permission_handle: RactorPermissionHandle,
    pub bus: EventBus<Event>,
}

/// Bundles everything needed to spawn the turn task.
pub struct TurnSetupInfo {
    pub built: BuiltProvider,
    pub emit: EmitFn,
    pub gate: PermissionGate,
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
            current_turn_token: None,
            current_gate: None,
            current_turn_handle: None,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            AgentMsg::Run { command } => self.run_turn(myself, state, command).await,
            AgentMsg::RunLeader { command } => {
                let cmd: AgentCommand = command.into();
                self.run_turn(myself, state, cmd).await;
            }
            AgentMsg::Abort => {
                handlers::abort_turn(state).await;
            }
            AgentMsg::TurnComplete => {
                handlers::complete_turn(state);
            }
        }
        Ok(())
    }
}

impl RactorAgentActor {
    async fn run_turn(
        &self,
        myself: ActorRef<<RactorAgentActor as ractor::Actor>::Msg>,
        state: &mut AgentActorState,
        mut command: AgentCommand,
    ) {
        if Self::reject_if_turn_in_flight(state, &command) {
            return;
        }

        let turn_info = match Self::build_provider_turn(state, &mut command).await {
            Ok(info) => info,
            Err(()) => return,
        };

        let handle = Self::spawn_turn_task(myself, turn_info, command);
        state.current_turn_handle = Some(handle);
    }

    /// Returns true if a turn is already in flight; emits error+Done and returns true.
    fn reject_if_turn_in_flight(state: &AgentActorState, command: &AgentCommand) -> bool {
        if state.current_turn_token.is_some() || state.current_turn_handle.is_some() {
            let id = command.id.clone();
            state.bus.publish(runie_core::Event::Error {
                id: id.clone(),
                message: "Turn already in flight, rejected new Run".into(),
            });
            state.bus.publish(runie_core::Event::Done { id });
            return true;
        }
        false
    }

    /// Builds the provider, sets up permission gate, and stores token/gate in state.
    /// Returns `Err(())` on provider build failure.
    pub(crate) async fn build_provider_turn(
        state: &mut AgentActorState,
        command: &mut AgentCommand,
    ) -> Result<TurnSetupInfo, ()> {
        let provider = state.provider_handle.clone();
        let permission = state.permission_handle.clone();

        let (provider_key, model) = if runie_core::provider::is_mock_enabled() {
            ("mock".to_owned(), "echo".to_owned())
        } else {
            (command.provider.clone(), command.model.clone())
        };

        let built = match provider.build(provider_key, model).await {
            Ok(b) => b,
            Err(e) => {
                Self::emit_error_and_done(state, &command.id, format!("Provider error: {e}"));
                return Err(());
            }
        };

        let emit = Self::create_emit_closure(state);

        let cancel_token = CancellationToken::new();
        command.cancellation_token = cancel_token.clone();
        let cancel_token_gate = cancel_token.clone();

        let gate = Self::create_permission_gate_with_cancel(permission.clone(), cancel_token_gate).await;

        // Store token and gate in state so Abort handler can cancel them.
        state.current_turn_token = Some(Arc::new(cancel_token));
        state.current_gate = Some(gate.clone());

        Ok(TurnSetupInfo { built, emit, gate, bus: state.bus.clone() })
    }

    /// Spawns the turn as a background task; sends TurnComplete to the actor on finish.
    pub(crate) fn spawn_turn_task(
        myself: ActorRef<<RactorAgentActor as ractor::Actor>::Msg>,
        info: TurnSetupInfo,
        command: AgentCommand,
    ) -> tokio::task::JoinHandle<()> {
        let emit_for_task = info.emit.clone();
        let command_id = command.id.clone();
        let bus_for_task = info.bus.clone();
        let myself_for_task = myself.clone();

        tokio::spawn(async move {
            let turn = run_agent_turn(
                &info.built,
                &command,
                emit_for_task,
                DEFAULT_MAX_TOOL_ROUNDS,
                info.gate.clone(),
            );
            tokio::pin!(turn);

            let result = turn.await;

            if let Err(e) = result {
                Self::publish_error_and_done(&bus_for_task, &command_id, format!("Agent error: {e}"));
            }

            // Notify actor to clear current_turn_* state.
            let _ = myself_for_task.send_message(AgentMsg::TurnComplete);
        })
    }

    pub(crate) fn publish_error_and_done(bus: &EventBus<Event>, id: &str, message: String) {
        bus.publish(runie_core::Event::Error { id: id.to_owned(), message });
        bus.publish(runie_core::Event::Done { id: id.to_owned() });
    }

    pub(crate) fn create_emit_closure(state: &AgentActorState) -> EmitFn {
        let bus = state.bus.clone();
        Arc::new(move |evt: Event| {
            bus.publish(evt);
        })
    }

    pub(crate) async fn create_permission_gate_with_cancel(
        permission_handle: RactorPermissionHandle,
        abort_tx: tokio_util::sync::CancellationToken,
    ) -> PermissionGate {
        handlers::create_permission_gate(permission_handle, abort_tx).await
    }

    pub(crate) fn emit_error_and_done(state: &mut AgentActorState, id: &str, message: String) {
        state
            .bus
            .publish(runie_core::Event::Error { id: id.to_owned(), message });
        state
            .bus
            .publish(runie_core::Event::Done { id: id.to_owned() });
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
    let args = RactorAgentArgs { provider_handle, permission_handle, bus: bus.clone() };
    spawn_ractor(None, RactorAgentActor, args).await
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
