//! `AgentStateActor` — the only mutator of agent state.
//!
//! External code sends `StateCommand`s; the worker task applies them in
//! order and updates a `watch::Sender<AgentStateSnapshot>` so readers can
//! observe the latest state without locking.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, watch};

use crate::events::EventBus;
use crate::types::{AgentEvent, AgentMessage, AgentTool, Model, ThinkingLevel};

use super::snapshot::AgentStateSnapshot;
use crate::task_owner::{spawn_actor_worker, TaskOwner};

/// Maximum number of in-flight commands the actor accepts before backpressure
/// kicks in. Sized to absorb a full assistant turn's worth of mutations.
const MAILBOX_CAPACITY: usize = 1024;

fn is_assistant(message: &AgentMessage) -> bool {
    matches!(message, AgentMessage::Assistant(_))
}

/// State-mutating commands. The actor owns the only `Sender`; the rest of
/// the codebase sends through handles.
pub enum StateCommand {
    SetSystemPrompt(String),
    SetModel(Model),
    SetThinkingLevel(ThinkingLevel),
    PushMessage(AgentMessage, Option<oneshot::Sender<()>>),
    ReplaceMessages(Vec<AgentMessage>),
    SetTools(Vec<Arc<dyn AgentTool>>),
    MarkStreaming(bool),
    SetStreamingMessage(Option<AgentMessage>),
    SetStreamingState {
        streaming: bool,
        message: Option<AgentMessage>,
    },
    AddPendingToolCall(String, Option<oneshot::Sender<()>>),
    RemovePendingToolCall(String, Option<oneshot::Sender<()>>),
    SetError(Option<String>, Option<oneshot::Sender<()>>),
    Reset,
}

/// Handle to the state actor. Cheap to clone (one mpsc sender).
#[derive(Clone)]
pub struct AgentStateActor {
    tx: mpsc::Sender<StateCommand>,
    snapshot_rx: watch::Receiver<AgentStateSnapshot>,
    _worker: Arc<TaskOwner>,
}

impl AgentStateActor {
    /// Spawn the actor worker on the current Tokio runtime.
    pub fn new() -> Self {
        let (snap_tx, snap_rx) = watch::channel(AgentStateSnapshot::default());

        // OWNER: AgentStateActor — the worker handle is retained by TaskOwner
        // and is aborted when the final actor handle is dropped.
        let (tx, worker) = spawn_actor_worker!(MAILBOX_CAPACITY, move |rx| async move {
            run_worker(rx, snap_tx).await;
        });

        Self {
            tx,
            snapshot_rx: snap_rx,
            _worker: worker,
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
        let _ = self.tx.send(StateCommand::PushMessage(m, None)).await;
    }

    async fn push_message_wait(&self, m: AgentMessage) {
        let (ack_tx, ack_rx) = oneshot::channel();
        if self
            .tx
            .send(StateCommand::PushMessage(m, Some(ack_tx)))
            .await
            .is_ok()
        {
            let _ = ack_rx.await;
        }
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

    pub async fn set_streaming_state(&self, streaming: bool, message: Option<AgentMessage>) {
        let _ = self
            .tx
            .send(StateCommand::SetStreamingState { streaming, message })
            .await;
    }

    pub async fn add_pending_tool_call(&self, id: String) {
        let (ack_tx, ack_rx) = oneshot::channel();
        if self
            .tx
            .send(StateCommand::AddPendingToolCall(id, Some(ack_tx)))
            .await
            .is_ok()
        {
            let _ = ack_rx.await;
        }
    }

    pub async fn remove_pending_tool_call(&self, id: String) {
        let (ack_tx, ack_rx) = oneshot::channel();
        if self
            .tx
            .send(StateCommand::RemovePendingToolCall(id, Some(ack_tx)))
            .await
            .is_ok()
        {
            let _ = ack_rx.await;
        }
    }

    pub async fn set_error(&self, e: Option<String>) {
        let (ack_tx, ack_rx) = oneshot::channel();
        if self
            .tx
            .send(StateCommand::SetError(e, Some(ack_tx)))
            .await
            .is_ok()
        {
            let _ = ack_rx.await;
        }
    }

    pub async fn reset(&self) {
        let _ = self.tx.send(StateCommand::Reset).await;
    }

    /// Apply a published agent event to the actor-owned projection.
    ///
    /// This is the single event-to-state boundary used by the loop driver;
    /// callers do not mutate projection fields directly.
    pub async fn apply_event(&self, event: &AgentEvent) {
        match event {
            AgentEvent::MessageStart { .. }
            | AgentEvent::MessageUpdate { .. }
            | AgentEvent::MessageEnd { .. } => self.apply_message_event(event).await,
            AgentEvent::ToolExecutionStart { .. } | AgentEvent::ToolExecutionEnd { .. } => {
                self.apply_tool_event(event).await
            }
            AgentEvent::Error { message } => {
                self.set_streaming_state(false, None).await;
                self.set_error(Some(message.clone())).await;
            }
            AgentEvent::ThinkingLevelChanged { level } => self.set_thinking_level(*level).await,
            AgentEvent::Reset => self.reset().await,
            AgentEvent::AgentStart
            | AgentEvent::AgentEnd { .. }
            | AgentEvent::TurnStart
            | AgentEvent::Waiting { .. }
            | AgentEvent::ThemeChanged { .. }
            | AgentEvent::ToolDisplayModeChanged { .. }
            | AgentEvent::TurnEnd { .. }
            | AgentEvent::ToolExecutionUpdate { .. }
            | AgentEvent::BackgroundWorkStarted { .. }
            | AgentEvent::BackgroundWorkProgress { .. }
            | AgentEvent::BackgroundWorkFinished { .. }
            | AgentEvent::BackgroundWorkCancelled { .. } => {}
        }
    }

    /// Publish an event and apply the same event to this actor-owned state
    /// projection before returning. State-changing callers use this boundary
    /// instead of independently publishing and mutating the projection.
    pub async fn publish_event(&self, bus: &EventBus, event: AgentEvent) {
        bus.publish(event.clone());
        self.apply_event(&event).await;
    }

    async fn apply_message_event(&self, event: &AgentEvent) {
        match event {
            AgentEvent::MessageStart { message } if is_assistant(message) => {
                self.set_streaming_state(true, Some(message.clone())).await;
            }
            AgentEvent::MessageUpdate { message, .. } => {
                self.set_streaming_message(Some(message.clone())).await;
            }
            AgentEvent::MessageEnd { message } => {
                self.push_message_wait(message.clone()).await;
                if let AgentMessage::Assistant(assistant) = message {
                    self.set_streaming_state(false, None).await;
                    self.set_error(assistant.error_message.clone()).await;
                }
            }
            _ => {}
        }
    }

    async fn apply_tool_event(&self, event: &AgentEvent) {
        match event {
            AgentEvent::ToolExecutionStart { tool_call_id, .. } => {
                self.add_pending_tool_call(tool_call_id.clone()).await;
            }
            AgentEvent::ToolExecutionEnd { tool_call_id, .. } => {
                self.remove_pending_tool_call(tool_call_id.clone()).await;
            }
            _ => {}
        }
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
        StateCommand::PushMessage(m, ack) => apply_push_message(state, m, ack),
        StateCommand::ReplaceMessages(msgs) => state.messages = msgs,
        StateCommand::SetTools(tools) => state.tools = tools,
        StateCommand::MarkStreaming(on) => state.is_streaming = on,
        StateCommand::SetStreamingMessage(m) => state.streaming_message = m,
        StateCommand::SetStreamingState { streaming, message } => {
            state.is_streaming = streaming;
            state.streaming_message = message;
        }
        StateCommand::AddPendingToolCall(id, ack) => {
            if !state.pending_tool_calls.contains(&id) {
                state.pending_tool_calls.push(id);
            }
            if let Some(ack) = ack {
                let _ = ack.send(());
            }
        }
        StateCommand::RemovePendingToolCall(id, ack) => {
            state.pending_tool_calls.retain(|x| x != &id);
            if let Some(ack) = ack {
                let _ = ack.send(());
            }
        }
        StateCommand::SetError(e, ack) => {
            state.error_message = e;
            if let Some(ack) = ack {
                let _ = ack.send(());
            }
        }
        StateCommand::Reset => {
            *state = AgentStateSnapshot::default();
        }
    }
}

fn apply_push_message(
    state: &mut AgentStateSnapshot,
    message: AgentMessage,
    ack: Option<oneshot::Sender<()>>,
) {
    state.messages.push(message);
    if let Some(ack) = ack {
        let _ = ack.send(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssistantMessage, StopReason, UserContent, UserMessage};

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

    #[tokio::test]
    async fn event_projection_owns_stream_and_terminal_error_transitions() {
        let actor = AgentStateActor::new();
        let assistant = AgentMessage::Assistant(AssistantMessage {
            stop_reason: Some(StopReason::Aborted),
            error_message: Some("aborted".into()),
            ..Default::default()
        });
        actor
            .apply_event(&AgentEvent::MessageStart {
                message: assistant.clone(),
            })
            .await;
        actor.sync().await;
        assert!(actor.snapshot().is_streaming);
        actor
            .apply_event(&AgentEvent::MessageEnd { message: assistant })
            .await;
        actor.sync().await;
        let snapshot = actor.snapshot();
        assert!(!snapshot.is_streaming);
        assert_eq!(snapshot.error_message.as_deref(), Some("aborted"));
        assert_eq!(snapshot.messages.len(), 1);
    }

    #[tokio::test]
    async fn publish_event_keeps_bus_and_projection_on_one_event_boundary() {
        let actor = AgentStateActor::new();
        let bus = EventBus::new();
        let mut events = bus.subscribe();
        let message = AgentMessage::User(UserMessage {
            content: vec![UserContent::Text { text: "hey".into() }],
            timestamp: 1,
        });

        actor
            .publish_event(&bus, AgentEvent::MessageEnd { message })
            .await;
        actor.sync().await;

        assert!(matches!(
            events.try_recv(),
            Ok(AgentEvent::MessageEnd { .. })
        ));
        assert_eq!(actor.snapshot().messages.len(), 1);
    }

    #[tokio::test]
    async fn error_event_owns_non_message_error_projection() {
        let actor = AgentStateActor::new();
        actor
            .apply_event(&AgentEvent::Error {
                message: "provider: no stream".into(),
            })
            .await;
        actor.sync().await;
        assert_eq!(
            actor.snapshot().error_message.as_deref(),
            Some("provider: no stream")
        );
    }
}
