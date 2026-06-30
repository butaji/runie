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

    pub async fn run(&self, command: AgentCommand) {
        let cmd = runie_core::actors::leader::LeaderAgentCmd {
            content: command.content,
            id: command.id,
            provider: command.provider,
            model: command.model,
            thinking_level: command.thinking_level,
            read_only: command.read_only,
            skills_context: command.skills_context,
            system_prompt: command.system_prompt,
        };
        self.inner.run(cmd).await;
    }

    pub async fn run_if_queued(&self, turn_handle: &RactorTurnHandle) {
        turn_handle
            .send(runie_core::actors::TurnMsg::RunIfQueued)
            .await;
    }
}

/// Box over agent-handle variants so UiActor can hold either type without
/// generics or async-fn trait objects.
pub enum AgentHandleBox {
    Actor(AgentActorHandle),
    Leader(LeaderAgentActorHandle),
}

impl AgentHandleBox {
    #[allow(dead_code)]
    pub async fn run(&self, command: AgentCommand) {
        match self {
            Self::Actor(h) => h.run(command).await,
            Self::Leader(h) => h.run(command).await,
        }
    }

    pub async fn run_if_queued(&self, turn: &RactorTurnHandle) {
        match self {
            Self::Actor(h) => h.run_if_queued(turn).await,
            Self::Leader(h) => h.run_if_queued(turn).await,
        }
    }
}
