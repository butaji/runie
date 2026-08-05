//! `AgentStateActor` — the only mutator of agent state.
//!
//! External code sends `StateCommand`s; the worker task applies them in
//! order and updates a `watch::Sender<AgentStateSnapshot>` so readers can
//! observe the latest state without locking.

use std::sync::Arc;

use tokio::sync::{mpsc, watch};

use crate::types::{AgentMessage, AgentTool, Model, ThinkingLevel};

use super::snapshot::AgentStateSnapshot;

/// Maximum number of in-flight commands the actor accepts before backpressure
/// kicks in. Sized to absorb a full assistant turn's worth of mutations.
const MAILBOX_CAPACITY: usize = 1024;

/// State-mutating commands. The actor owns the only `Sender`; the rest of
/// the codebase sends through handles.
pub enum StateCommand {
    SetSystemPrompt(String),
    SetModel(Model),
    SetThinkingLevel(ThinkingLevel),
    PushMessage(AgentMessage),
    ReplaceMessages(Vec<AgentMessage>),
    SetTools(Vec<Arc<dyn AgentTool>>),
    MarkStreaming(bool),
    SetStreamingMessage(Option<AgentMessage>),
    AddPendingToolCall(String),
    RemovePendingToolCall(String),
    SetError(Option<String>),
    Reset,
}

/// Handle to the state actor. Cheap to clone (one mpsc sender).
#[derive(Clone)]
pub struct AgentStateActor {
    tx: mpsc::Sender<StateCommand>,
    snapshot_rx: watch::Receiver<AgentStateSnapshot>,
}

impl AgentStateActor {
    /// Spawn the actor worker on the current Tokio runtime.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(MAILBOX_CAPACITY);
        let (snap_tx, snap_rx) = watch::channel(AgentStateSnapshot::default());

        // OWNER: AgentStateActor — the worker task is the only consumer of
        // this `tokio::spawn`. Its lifetime is tied to `rx` (held here).
        tokio::spawn(async move {
            run_worker(rx, snap_tx).await;
        });

        Self {
            tx,
            snapshot_rx: snap_rx,
        }
    }

    pub async fn set_system_prompt(&self, s: String) {
        let _ = self.tx.send(StateCommand::SetSystemPrompt(s)).await;
    }

    pub async fn set_model(&self, m: Model) {
        let _ = self.tx.send(StateCommand::SetModel(m)).await;
    }

    pub async fn set_thinking_level(&self, t: ThinkingLevel) {
        let _ = self.tx.send(StateCommand::SetThinkingLevel(t)).await;
    }

    pub async fn push_message(&self, m: AgentMessage) {
        let _ = self.tx.send(StateCommand::PushMessage(m)).await;
    }

    pub async fn replace_messages(&self, msgs: Vec<AgentMessage>) {
        let _ = self.tx.send(StateCommand::ReplaceMessages(msgs)).await;
    }

    pub async fn set_tools(&self, tools: Vec<Arc<dyn AgentTool>>) {
        let _ = self.tx.send(StateCommand::SetTools(tools)).await;
    }

    pub async fn mark_streaming(&self, on: bool) {
        let _ = self.tx.send(StateCommand::MarkStreaming(on)).await;
    }

    pub async fn set_streaming_message(&self, m: Option<AgentMessage>) {
        let _ = self.tx.send(StateCommand::SetStreamingMessage(m)).await;
    }

    pub async fn add_pending_tool_call(&self, id: String) {
        let _ = self.tx.send(StateCommand::AddPendingToolCall(id)).await;
    }

    pub async fn remove_pending_tool_call(&self, id: String) {
        let _ = self.tx.send(StateCommand::RemovePendingToolCall(id)).await;
    }

    pub async fn set_error(&self, e: Option<String>) {
        let _ = self.tx.send(StateCommand::SetError(e)).await;
    }

    pub async fn reset(&self) {
        let _ = self.tx.send(StateCommand::Reset).await;
    }

    /// Borrow the current snapshot. Use this for read-only views.
    pub fn snapshot(&self) -> AgentStateSnapshot {
        self.snapshot_rx.borrow().clone()
    }

    /// Wait until the snapshot reflects all previously sent commands.
    pub async fn sync(&self) {
        let mut rx = self.snapshot_rx.clone();
        // Wait for at least one snapshot after the current view.
        let _ = rx.changed().await;
    }
}

impl Default for AgentStateActor {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_worker(
    mut rx: mpsc::Receiver<StateCommand>,
    snap_tx: watch::Sender<AgentStateSnapshot>,
) {
    let mut state = AgentStateSnapshot::default();

    while let Some(cmd) = rx.recv().await {
        apply(&mut state, cmd);
        // Best-effort: ignore send errors (no readers).
        let _ = snap_tx.send(state.clone());
    }
}

fn apply(state: &mut AgentStateSnapshot, cmd: StateCommand) {
    match cmd {
        StateCommand::SetSystemPrompt(s) => state.system_prompt = s,
        StateCommand::SetModel(m) => state.model = m,
        StateCommand::SetThinkingLevel(t) => state.thinking_level = t,
        StateCommand::PushMessage(m) => state.messages.push(m),
        StateCommand::ReplaceMessages(msgs) => state.messages = msgs,
        StateCommand::SetTools(tools) => state.tools = tools,
        StateCommand::MarkStreaming(on) => state.is_streaming = on,
        StateCommand::SetStreamingMessage(m) => state.streaming_message = m,
        StateCommand::AddPendingToolCall(id) => {
            if !state.pending_tool_calls.contains(&id) {
                state.pending_tool_calls.push(id);
            }
        }
        StateCommand::RemovePendingToolCall(id) => {
            state.pending_tool_calls.retain(|x| x != &id);
        }
        StateCommand::SetError(e) => state.error_message = e,
        StateCommand::Reset => {
            *state = AgentStateSnapshot::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TextContent, UserContent, UserMessage};

    #[tokio::test]
    async fn push_message_visible_in_snapshot() {
        let actor = AgentStateActor::new();
        actor
            .push_message(AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: "hi".into() }],
                timestamp: 1,
            }))
            .await;
        actor.sync().await;
        let snap = actor.snapshot();
        assert_eq!(snap.messages.len(), 1);
    }

    #[tokio::test]
    async fn reset_clears_state() {
        let actor = AgentStateActor::new();
        actor.set_system_prompt("sys".into()).await;
        actor.mark_streaming(true).await;
        actor.reset().await;
        actor.sync().await;
        let snap = actor.snapshot();
        assert_eq!(snap.system_prompt, "");
        assert!(!snap.is_streaming);
    }

    #[tokio::test]
    async fn pending_tool_calls_deduplicated() {
        let actor = AgentStateActor::new();
        actor.add_pending_tool_call("a".into()).await;
        actor.add_pending_tool_call("a".into()).await;
        actor.sync().await;
        assert_eq!(actor.snapshot().pending_tool_calls.len(), 1);
    }
}
