//! Agent handle types for UiActor.
//!
//! UiActor can be backed by either a direct mpsc channel or a LeaderAgentHandle.

use runie_agent::{AgentCommand, AgentMsg};
use runie_core::actors::RactorTurnHandle;

/// Simple handle for sending commands to the agent.
#[derive(Clone)]
pub struct AgentActorHandle {
    tx: tokio::sync::mpsc::Sender<AgentMsg>,
}

impl AgentActorHandle {
    pub fn new(tx: tokio::sync::mpsc::Sender<AgentMsg>) -> Self {
        Self { tx }
    }

    pub async fn run(&self, command: AgentCommand) {
        let _ = self.tx.send(AgentMsg::Run { command }).await;
    }

    pub async fn abort(&self) {
        let _ = self.tx.send(AgentMsg::Abort).await;
    }

    pub async fn run_if_queued(&self, turn_handle: &RactorTurnHandle) {
        turn_handle
            .send(runie_core::actors::TurnMsg::RunIfQueued)
            .await;
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

    /// Create a no-op handle for use before `Leader::start_with_bus()` returns.
    pub fn new_noop() -> Self {
        Self {
            inner: std::sync::Arc::new(NoOpAgentHandle),
        }
    }

    pub async fn run(&self, command: AgentCommand) {
        let cmd = runie_core::actors::leader::LeaderAgentCmd {
            content: command.content.clone(),
            id: command.id.clone(),
            provider: command.provider.clone(),
            model: command.model.clone(),
            thinking_level: command.thinking_level,
            read_only: command.read_only,
            skills_context: command.skills_context.clone(),
            system_prompt: command.system_prompt.clone(),
        };
        self.inner.run(cmd).await;
    }

    pub async fn abort(&self) {
        self.inner.abort().await;
    }

    pub async fn run_if_queued(&self, turn_handle: &RactorTurnHandle) {
        turn_handle
            .send(runie_core::actors::TurnMsg::RunIfQueued)
            .await;
    }
}

/// No-op agent handle used during early startup before the real agent is available.
/// All methods are no-ops; call `LeaderAgentActorHandle::new()` after `Leader::start_with_bus()`.
#[derive(Clone, Default)]
pub struct NoOpAgentHandle;

impl runie_core::actors::leader::LeaderAgentHandle for NoOpAgentHandle {
    fn run(
        &self,
        _cmd: runie_core::actors::leader::LeaderAgentCmd,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
    fn abort(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async {})
    }
}

/// Box over agent-handle variants so UiActor can hold either type without
/// generics or async-fn trait objects.
pub enum AgentHandleBox {
    Actor(AgentActorHandle),
    Leader(LeaderAgentActorHandle),
}

impl AgentHandleBox {
    /// Direct agent run (bypasses queue). Kept for future agent-control use.
    #[allow(dead_code, reason = "kept for future direct agent control")]
    pub async fn run(&self, command: AgentCommand) {
        match self {
            Self::Actor(h) => h.run(command).await,
            Self::Leader(h) => h.run(command).await,
        }
    }

    /// Send abort to cancel the currently running turn.
    pub async fn abort(&self) {
        match self {
            Self::Actor(h) => h.abort().await,
            Self::Leader(h) => h.abort().await,
        }
    }

    pub async fn run_if_queued(&self, turn: &RactorTurnHandle) {
        match self {
            Self::Actor(h) => h.run_if_queued(turn).await,
            Self::Leader(h) => h.run_if_queued(turn).await,
        }
    }
}
