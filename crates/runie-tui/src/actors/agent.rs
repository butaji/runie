//! AgentActor — handles LLM API calls.
//!
//! Phase 4: Shell implementation. Full streaming implementation in later phase.

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::Actor;

/// Agent message — commands to the agent actor
#[derive(Debug, Clone)]
pub enum AgentMsg {
    /// Start an agent turn with a prompt
    Start {
        prompt: String,
        model: String,
    },
    /// Cancel the current agent operation
    Cancel,
}

/// Agent actor — handles LLM API calls and streaming responses.
///
/// Phase 4: Shell that accepts messages but doesn't yet process them.
/// Full implementation will use runie-ai for API calls.
pub struct AgentActor;

impl AgentActor {
    pub fn new() -> Self {
        Self
    }
}

impl Actor for AgentActor {
    type Msg = AgentMsg;

    fn name(&self) -> &'static str {
        "agent"
    }

    async fn run(self, _msg_tx: mpsc::Sender<AgentMsg>, cancel: CancellationToken) {
        tracing::info!(target: "runie", "[ACTOR:agent] AgentActor starting (shell)");

        // Wait for cancellation
        cancel.cancelled().await;

        tracing::info!(target: "runie", "[ACTOR:agent] AgentActor stopped");
    }
}

impl Default for AgentActor {
    fn default() -> Self {
        Self::new()
    }
}
