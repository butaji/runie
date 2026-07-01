//! `AgentActor` — ractor-based implementation.
//!
//! This is the production implementation of the AgentActor using ractor.

use std::pin::Pin;
use std::sync::Arc;

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
use crate::stream_response::EmitFn;
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
    /// Abort the currently running turn, if any.
    Abort,
    /// Internal: the spawned turn task finished (success or error).
    /// Clears the in-flight turn state so a subsequent Run is accepted.
    TurnComplete,
}

// ── Ractor-based AgentActor ───────────────────────────────────────────────────

/// Ractor State for AgentActor — holds provider and permission handles.
struct AgentActorState {
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
struct TurnSetupInfo {
    built: Arc<dyn runie_core::provider::Provider>,
    emit: EmitFn,
    gate: PermissionGate,
    bus: EventBus<Event>,
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
                // Take and cancel the current turn token and gate.
                if let Some(token) = state.current_turn_token.take() {
                    token.cancel();
                }
                if let Some(gate) = state.current_gate.take() {
                    gate.cancel_pending();
                }
                // Abort and await the old turn handle.
                if let Some(handle) = state.current_turn_handle.take() {
                    handle.abort();
                    let _ = handle.await;
                }
            }
            AgentMsg::TurnComplete => {
                // Normal turn completion: clear all in-flight state so a subsequent
                // Run is accepted.  Do NOT cancel the token/gate — the turn finished
                // on its own.
                state.current_turn_token = None;
                state.current_gate = None;
                state.current_turn_handle = None;
            }
        }
        Ok(())
    }
}

impl RactorAgentActor {
    async fn run_turn(
        &self,
        myself: ActorRef<Self::Msg>,
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
                id,
                message: "Turn already in flight, rejected new Run".into(),
            });
            state.bus.publish(runie_core::Event::Done { id: command.id });
            return true;
        }
        false
    }

    /// Builds the provider, sets up permission gate, and stores token/gate in state.
    /// Returns `Err(())` on provider build failure.
    async fn build_provider_turn(
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

        let gate =
            Self::create_permission_gate_with_cancel(permission.clone(), cancel_token_gate).await;

        // Store token and gate in state so Abort handler can cancel them.
        state.current_turn_token = Some(Arc::new(cancel_token));
        state.current_gate = Some(gate.clone());

        Ok(TurnSetupInfo { built, emit, gate, bus: state.bus.clone() })
    }

    /// Spawns the turn as a background task; sends TurnComplete to the actor on finish.
    fn spawn_turn_task(
        myself: ActorRef<Self::Msg>,
        info: TurnSetupInfo,
        command: AgentCommand,
    ) -> tokio::task::JoinHandle<()> {
        let emit_for_task = info.emit.clone();
        let command_id = command.id.clone();
        let bus_for_task = info.bus.clone();
        let myself_for_task = myself.clone();

        tokio::spawn(async move {
            let turn = run_agent_turn(&info.built, &command, emit_for_task, 5, info.gate.clone());
            tokio::pin!(turn);

            let result = turn.await;

            if let Err(e) = result {
                let _ = Self::publish_error_and_done(&bus_for_task, &command_id, format!("Agent error: {e}"));
            }

            // Notify actor to clear current_turn_* state.
            let _ = myself_for_task.send_message(AgentMsg::TurnComplete);
        })
    }

    fn publish_error_and_done(bus: &EventBus<Event>, id: &str, message: String) {
        bus.publish(runie_core::Event::Error {
            id: id.to_owned(),
            message,
        });
        bus.publish(runie_core::Event::Done { id: id.to_owned() });
    }

    fn create_emit_closure(state: &AgentActorState) -> EmitFn {
        let bus = state.bus.clone();
        Arc::new(move |evt: Event| {
            bus.publish(evt);
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::Stream;
    use runie_core::actors::permission::RactorPermissionActor;
    use runie_core::actors::provider::ProviderFactory;
    use runie_core::config::Config;
    use runie_core::event::Event;
    use runie_core::message::ChatMessage;
    use runie_core::provider::{BuiltProvider, Provider, ProviderError};
    use runie_core::provider_event::{ProviderEvent, StopReason};
    use std::pin::Pin;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::timeout;

    /// A provider that immediately returns one text chunk then finishes.
    struct SimpleTextProvider;

    impl Provider for SimpleTextProvider {
        fn generate(
            &self,
            _messages: Vec<ChatMessage>,
        ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
            let stream = futures::stream::iter([
                Ok(ProviderEvent::TextDelta("hello".into())),
                Ok(ProviderEvent::Usage { input_tokens: 1, output_tokens: 1 }),
                Ok(ProviderEvent::Finish(StopReason::EndTurn)),
            ]);
            Box::pin(stream)
        }
    }

    struct TestFactory;

    impl ProviderFactory for TestFactory {
        fn build(
            &self,
            _provider: &str,
            _model: &str,
            _config: &Config,
        ) -> Pin<Box<dyn futures::Future<Output = Result<BuiltProvider, ProviderError>> + Send + '_>> {
            Box::pin(async move {
                Ok(BuiltProvider::new(
                    Box::new(SimpleTextProvider),
                    "test".into(),
                    "test-model".into(),
                ))
            })
        }

        fn validate_credentials(
            &self,
            _provider: &str,
            _model: &str,
            _config: &Config,
        ) -> Pin<Box<dyn futures::Future<Output = Result<Vec<String>, String>> + Send + '_>> {
            Box::pin(async move { Ok(vec!["test-model".into()]) })
        }
    }

    #[tokio::test]
    async fn agent_actor_accepts_second_turn_after_first_completes() {
        let bus = EventBus::<Event>::new(10);

        let (provider_handle, _, _) =
            runie_core::actors::provider::RactorProviderActor::spawn(
                bus.clone(),
                runie_core::actors::RactorConfigHandle::default(),
                Arc::new(TestFactory),
            )
            .await
            .unwrap();

        let (permission_handle, _, _) =
            RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

        let (agent_handle, _, _) =
            spawn_ractor_agent(bus.clone(), provider_handle, permission_handle)
                .await
                .unwrap();

        let mut sub = bus.subscribe();

        // --- First turn ---
        let cmd1 = AgentCommand {
            content: vec![ChatMessage::user("hello")],
            id: "turn-1".into(),
            provider: "test".into(),
            model: "test-model".into(),
            thinking_level: runie_core::model::ThinkingLevel::Default,
            read_only: false,
            skills_context: None,
            system_prompt: None,
            truncation: crate::truncate::TruncationPolicy::default(),
            cancellation_token: CancellationToken::new(),
        };
        agent_handle.send_message(AgentMsg::Run { command: cmd1 });

        // Wait for first turn to complete.
        timeout(Duration::from_secs(5), async {
            while let Ok(evt) = sub.recv().await {
                if matches!(evt, Event::Done { id } if id == "turn-1") {
                    return;
                }
            }
        })
        .await
        .unwrap();

        // Give TurnComplete message time to be processed by the actor.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // --- Second turn ---
        let cmd2 = AgentCommand {
            content: vec![ChatMessage::user("hello again")],
            id: "turn-2".into(),
            provider: "test".into(),
            model: "test-model".into(),
            thinking_level: runie_core::model::ThinkingLevel::Default,
            read_only: false,
            skills_context: None,
            system_prompt: None,
            truncation: crate::truncate::TruncationPolicy::default(),
            cancellation_token: CancellationToken::new(),
        };
        agent_handle.send_message(AgentMsg::Run { command: cmd2 });

        // Wait for second turn to complete.
        let mut turn2_done = false;
        let mut turn2_error = false;
        timeout(Duration::from_secs(5), async {
            while let Ok(evt) = sub.recv().await {
                if matches!(evt, Event::Done { id } if id == "turn-2") {
                    turn2_done = true;
                    break;
                }
                if matches!(evt, Event::Error { id, .. } if id == "turn-2") {
                    turn2_error = true;
                    break;
                }
            }
        })
        .await
        .unwrap();

        assert!(
            turn2_done,
            "second turn must complete; got error={turn2_error}"
        );
        assert!(
            !turn2_error,
            "second turn must not emit an Error event"
        );
    }
}
