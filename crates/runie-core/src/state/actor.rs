//! `AgentStateActor` — the only mutator of agent state.
//!
//! External code sends `StateCommand`s; the worker task applies them in
//! order and updates a `watch::Sender<AgentStateSnapshot>` so readers can
//! observe the latest state without locking.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, watch};

use crate::events::EventBus;
use crate::types::{AgentEvent, AgentMessage, AgentTool, Model, ThinkingLevel};

use super::snapshot::{AgentStateSnapshot, BackgroundWorkSnapshot, WorkflowSnapshot};
use crate::task_owner::{mailbox_ack, spawn_actor_worker, TaskOwner};

/// Maximum number of in-flight commands the actor accepts before backpressure
/// kicks in. Sized to absorb a full assistant turn's worth of mutations.
const MAILBOX_CAPACITY: usize = 1024;

fn is_assistant(message: &AgentMessage) -> bool {
    matches!(message, AgentMessage::Assistant(_))
}

/// State-mutating commands. The actor owns the only `Sender`; the rest of
/// the codebase sends through handles.
pub enum StateCommand {
    SetSystemPrompt(String, oneshot::Sender<()>),
    SetModel(Model, oneshot::Sender<()>),
    SetThinkingLevel(ThinkingLevel, oneshot::Sender<()>),
    PushMessage(AgentMessage, Option<oneshot::Sender<()>>),
    ReplaceMessages(Vec<AgentMessage>, oneshot::Sender<()>),
    SetTools(Vec<Arc<dyn AgentTool>>, oneshot::Sender<()>),
    MarkStreaming(bool, oneshot::Sender<()>),
    SetStreamingMessage(Option<AgentMessage>, oneshot::Sender<()>),
    SetStreamingState {
        streaming: bool,
        message: Option<AgentMessage>,
        ack: oneshot::Sender<()>,
    },
    AddPendingToolCall(String, Option<oneshot::Sender<()>>),
    RemovePendingToolCall(String, Option<oneshot::Sender<()>>),
    SetError(Option<String>, Option<oneshot::Sender<()>>),
    ApplyEvent(Box<AgentEvent>, oneshot::Sender<()>),
    Reset(oneshot::Sender<()>),
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

    /// Shared DSL boundary for acknowledged state commands. The constructor
    /// remains supplied by each method so payload semantics stay visible.
    async fn acknowledge<F>(&self, command: F)
    where
        F: FnOnce(oneshot::Sender<()>) -> StateCommand,
    {
        let _ = mailbox_ack!(self.tx, command);
    }

    pub async fn set_system_prompt(&self, s: String) {
        self.acknowledge(|reply| StateCommand::SetSystemPrompt(s, reply))
            .await;
    }

    pub async fn set_model(&self, m: Model) {
        self.acknowledge(|reply| StateCommand::SetModel(m, reply))
            .await;
    }

    pub async fn set_thinking_level(&self, t: ThinkingLevel) {
        self.acknowledge(|reply| StateCommand::SetThinkingLevel(t, reply))
            .await;
    }

    pub async fn push_message(&self, m: AgentMessage) {
        self.acknowledge(|reply| StateCommand::PushMessage(m, Some(reply)))
            .await;
    }

    pub async fn replace_messages(&self, msgs: Vec<AgentMessage>) {
        self.acknowledge(|reply| StateCommand::ReplaceMessages(msgs, reply))
            .await;
    }

    pub async fn set_tools(&self, tools: Vec<Arc<dyn AgentTool>>) {
        self.acknowledge(|reply| StateCommand::SetTools(tools, reply))
            .await;
    }

    pub async fn mark_streaming(&self, on: bool) {
        self.acknowledge(|reply| StateCommand::MarkStreaming(on, reply))
            .await;
    }

    pub async fn set_streaming_message(&self, m: Option<AgentMessage>) {
        self.acknowledge(|reply| StateCommand::SetStreamingMessage(m, reply))
            .await;
    }

    pub async fn set_streaming_state(&self, streaming: bool, message: Option<AgentMessage>) {
        self.acknowledge(|reply| StateCommand::SetStreamingState {
            streaming,
            message,
            ack: reply,
        })
        .await;
    }

    pub async fn add_pending_tool_call(&self, id: String) {
        self.acknowledge(|reply| StateCommand::AddPendingToolCall(id, Some(reply)))
            .await;
    }

    pub async fn remove_pending_tool_call(&self, id: String) {
        self.acknowledge(|reply| StateCommand::RemovePendingToolCall(id, Some(reply)))
            .await;
    }

    pub async fn set_error(&self, e: Option<String>) {
        self.acknowledge(|reply| StateCommand::SetError(e, Some(reply)))
            .await;
    }

    pub async fn reset(&self) {
        self.acknowledge(StateCommand::Reset).await;
    }

    /// Apply a published agent event to the actor-owned projection.
    ///
    /// This is the single event-to-state boundary used by the loop driver;
    /// callers do not mutate projection fields directly.
    pub async fn apply_event(&self, event: &AgentEvent) {
        self.acknowledge(|reply| StateCommand::ApplyEvent(Box::new(event.clone()), reply))
            .await;
    }

    /// Publish an event and apply the same event to this actor-owned state
    /// projection before returning. State-changing callers use this boundary
    /// instead of independently publishing and mutating the projection.
    pub async fn publish_event(&self, bus: &EventBus, event: AgentEvent) {
        bus.publish(event.clone());
        self.apply_event(&event).await;
    }

    /// Publish and reduce a Pi-core event through the closed typed boundary.
    ///
    /// Keeping this separate from `publish_event` makes it impossible for a
    /// Pi loop path to accidentally emit a Runie/TUI-only event while still
    /// applying the compatibility representation to this actor's projection.
    pub async fn publish_pi_event(&self, bus: &EventBus, event: crate::pi_event::PiAgentEvent) {
        let wire_event = event.clone();
        let event = event.try_into_agent_event();
        bus.publish_pi(wire_event);
        self.apply_event(&event).await;
    }

    #[allow(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
        reason = "the event reducer keeps every state transition explicit"
    )]
    fn apply_event_to_state(state: &mut AgentStateSnapshot, event: AgentEvent) {
        match event {
            AgentEvent::AgentStart => {
                state.is_streaming = true;
                state.streaming_message = None;
                state.pending_tool_calls.clear();
                state.error_message = None;
            }
            AgentEvent::ModelChanged { model } => {
                state.model = model;
            }
            AgentEvent::ActiveToolsChanged { .. } => {
                // Session configuration is reduced by SessionActor; the
                // agent message/state projection has no active-tool field.
            }
            AgentEvent::SessionLabelChanged { .. } => {
                // SessionActor owns label journal facts.
            }
            AgentEvent::SessionNameChanged { .. } => {
                // SessionActor owns session metadata facts.
            }
            AgentEvent::BranchSummaryCreated { .. } => {
                // SessionActor owns branch-summary journal records.
            }
            AgentEvent::CustomSessionEntryCreated { .. } => {
                // Extension-owned session data belongs to SessionActor.
            }
            AgentEvent::CompactionCreated { .. } => {
                // Compaction journal facts belong to SessionActor.
            }
            AgentEvent::OperationRecordCreated { .. } => {
                // Operation-lane journal facts belong to SessionActor.
            }
            AgentEvent::MessageStart { message } if is_assistant(&message) => {
                state.is_streaming = true;
                state.streaming_message = Some(message);
            }
            AgentEvent::MessageUpdate { message, .. } => {
                state.streaming_message = Some(message);
            }
            AgentEvent::MessageEnd { message } => {
                state.messages.push(message.clone());
                if matches!(message, AgentMessage::Assistant(_)) {
                    // Pi keeps `isStreaming` true until `agent_end` has
                    // settled. `message_end` closes the assistant message,
                    // but the agent may still run turn-end hooks and queue
                    // work before the run is truly idle.
                    state.streaming_message = None;
                }
            }
            AgentEvent::ToolExecutionStart { tool_call_id, .. } => {
                if !state.pending_tool_calls.contains(&tool_call_id) {
                    state.pending_tool_calls.push(tool_call_id);
                }
            }
            AgentEvent::ToolExecutionEnd { tool_call_id, .. } => {
                state.pending_tool_calls.retain(|id| id != &tool_call_id);
            }
            AgentEvent::WorkflowStarted {
                run_id,
                name,
                objective,
            } => {
                state.workflows.insert(
                    run_id,
                    WorkflowSnapshot {
                        name,
                        objective,
                        status: "active".into(),
                        ..WorkflowSnapshot::default()
                    },
                );
            }
            AgentEvent::WorkflowProgress {
                run_id,
                phase,
                state: phase_state,
                active_agents,
            } => {
                if let Some(workflow) = state.workflows.get_mut(&run_id) {
                    workflow.phase = Some(phase);
                    workflow.state = Some(phase_state);
                    workflow.active_agents = active_agents;
                }
            }
            AgentEvent::WorkflowFinished {
                run_id,
                status,
                elapsed_ms,
            } => {
                if let Some(workflow) = state.workflows.get_mut(&run_id) {
                    workflow.status = status;
                    workflow.elapsed_ms = elapsed_ms;
                }
            }
            AgentEvent::BackgroundWorkStarted {
                work_id,
                description,
                background,
            } => {
                state.background_work.insert(
                    work_id,
                    BackgroundWorkSnapshot {
                        description,
                        background,
                        status: "running".into(),
                        ..BackgroundWorkSnapshot::default()
                    },
                );
            }
            AgentEvent::BackgroundWorkProgress {
                work_id,
                description,
                activity,
            } => {
                let work = state.background_work.entry(work_id).or_default();
                work.description = description;
                work.activity = Some(activity);
                work.status = "running".into();
            }
            AgentEvent::BackgroundWorkFinished {
                work_id,
                description,
                is_error,
                elapsed_ms,
                error,
            } => {
                let work = state.background_work.entry(work_id).or_default();
                work.description = description;
                work.status = if is_error { "failed" } else { "done" }.into();
                work.elapsed_ms = elapsed_ms;
                work.error = error;
            }
            AgentEvent::BackgroundWorkCancelled {
                work_id,
                description,
                elapsed_ms,
            } => {
                let work = state.background_work.entry(work_id).or_default();
                work.description = description;
                work.status = "cancelled".into();
                work.elapsed_ms = elapsed_ms;
            }
            AgentEvent::Error { message } => {
                state.is_streaming = false;
                state.streaming_message = None;
                state.error_message = Some(message);
            }
            AgentEvent::ThinkingLevelChanged { level } => state.thinking_level = level,
            AgentEvent::TurnEnd { message, .. } => {
                if let AgentMessage::Assistant(assistant) = message {
                    if assistant.error_message.is_some() {
                        state.error_message = assistant.error_message.clone();
                    }
                }
            }
            AgentEvent::Reset => *state = AgentStateSnapshot::default(),
            AgentEvent::MessageStart { .. }
            | AgentEvent::TurnStart
            | AgentEvent::Waiting { .. }
            | AgentEvent::ThemeChanged { .. }
            | AgentEvent::ToolDisplayModeChanged { .. }
            | AgentEvent::ToolExecutionUpdate { .. } => {}
            AgentEvent::AgentEnd { .. } => {
                state.is_streaming = false;
                state.streaming_message = None;
                // Pi's run finalizer clears the runtime-owned pending set at
                // settlement, including interrupted tool batches.
                state.pending_tool_calls.clear();
            }
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

#[allow(
    clippy::too_many_lines,
    reason = "the actor command reducer keeps mailbox ownership explicit"
)]
fn apply(state: &mut AgentStateSnapshot, cmd: StateCommand) {
    match cmd {
        StateCommand::SetSystemPrompt(s, ack) => {
            state.system_prompt = s;
            let _ = ack.send(());
        }
        StateCommand::SetModel(m, ack) => {
            state.model = m;
            let _ = ack.send(());
        }
        StateCommand::SetThinkingLevel(t, ack) => {
            state.thinking_level = t;
            let _ = ack.send(());
        }
        StateCommand::PushMessage(m, ack) => apply_push_message(state, m, ack),
        StateCommand::ReplaceMessages(msgs, ack) => {
            state.messages = msgs;
            let _ = ack.send(());
        }
        StateCommand::SetTools(tools, ack) => {
            state.tools = tools;
            let _ = ack.send(());
        }
        StateCommand::MarkStreaming(on, ack) => {
            state.is_streaming = on;
            let _ = ack.send(());
        }
        StateCommand::SetStreamingMessage(m, ack) => {
            state.streaming_message = m;
            let _ = ack.send(());
        }
        StateCommand::SetStreamingState {
            streaming,
            message,
            ack,
        } => {
            state.is_streaming = streaming;
            state.streaming_message = message;
            let _ = ack.send(());
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
        StateCommand::ApplyEvent(event, ack) => {
            AgentStateActor::apply_event_to_state(state, *event);
            let _ = ack.send(());
        }
        StateCommand::Reset(ack) => {
            *state = AgentStateSnapshot::default();
            let _ = ack.send(());
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
    async fn replace_messages_acknowledges_before_returning() {
        let actor = AgentStateActor::new();
        actor
            .replace_messages(vec![AgentMessage::User(UserMessage {
                content: vec![UserContent::Text {
                    text: "restored".into(),
                }],
                timestamp: 1,
            })])
            .await;
        assert_eq!(actor.snapshot().messages.len(), 1);
        assert_eq!(actor.snapshot().messages[0].timestamp(), 1);
    }

    #[tokio::test]
    async fn model_changed_event_updates_the_owned_model_projection() {
        let actor = AgentStateActor::new();
        let model = Model {
            id: "model-1".into(),
            context_window: 42_000,
            ..Model::default()
        };
        actor
            .apply_event(&AgentEvent::ModelChanged {
                model: model.clone(),
            })
            .await;
        assert_eq!(actor.snapshot().model, model);
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
    async fn workflow_lifecycle_is_owned_by_core_snapshot() {
        let actor = AgentStateActor::new();
        actor
            .apply_event(&AgentEvent::WorkflowStarted {
                run_id: "wf-1".into(),
                name: "release".into(),
                objective: "ship it".into(),
            })
            .await;
        actor
            .apply_event(&AgentEvent::WorkflowProgress {
                run_id: "wf-1".into(),
                phase: "tests".into(),
                state: "active".into(),
                active_agents: 2,
            })
            .await;
        actor
            .apply_event(&AgentEvent::WorkflowFinished {
                run_id: "wf-1".into(),
                status: "done".into(),
                elapsed_ms: Some(1_200),
            })
            .await;
        actor.sync().await;
        let workflow = actor.snapshot().workflows.remove("wf-1").unwrap();
        assert_eq!(workflow.name, "release");
        assert_eq!(workflow.phase.as_deref(), Some("tests"));
        assert_eq!(workflow.active_agents, 2);
        assert_eq!(workflow.status, "done");
        assert_eq!(workflow.elapsed_ms, Some(1_200));
    }

    #[tokio::test]
    async fn background_work_lifecycle_is_owned_by_core_snapshot() {
        let actor = AgentStateActor::new();
        actor
            .apply_event(&AgentEvent::BackgroundWorkStarted {
                work_id: "bg-1".into(),
                description: "index files".into(),
                background: true,
            })
            .await;
        actor
            .apply_event(&AgentEvent::BackgroundWorkProgress {
                work_id: "bg-1".into(),
                description: "index files".into(),
                activity: "scanning src".into(),
            })
            .await;
        actor
            .apply_event(&AgentEvent::BackgroundWorkFinished {
                work_id: "bg-1".into(),
                description: "index files".into(),
                is_error: false,
                elapsed_ms: Some(900),
                error: None,
            })
            .await;
        actor.sync().await;
        let work = actor.snapshot().background_work.remove("bg-1").unwrap();
        assert_eq!(work.description, "index files");
        assert_eq!(work.activity.as_deref(), Some("scanning src"));
        assert!(work.background);
        assert_eq!(work.status, "done");
        assert_eq!(work.elapsed_ms, Some(900));
        assert_eq!(work.error, None);
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
        assert!(snapshot.is_streaming);
        assert!(snapshot.error_message.is_none());
        assert_eq!(snapshot.messages.len(), 1);
        actor
            .apply_event(&AgentEvent::TurnEnd {
                message: AgentMessage::Assistant(AssistantMessage {
                    error_message: Some("aborted".into()),
                    ..Default::default()
                }),
                tool_results: vec![],
            })
            .await;
        actor.sync().await;
        assert_eq!(actor.snapshot().error_message.as_deref(), Some("aborted"));
        actor
            .apply_event(&AgentEvent::AgentEnd { messages: vec![] })
            .await;
        actor.sync().await;
        assert!(!actor.snapshot().is_streaming);
    }

    #[tokio::test]
    async fn agent_start_reopens_stream_and_clears_previous_error() {
        let actor = AgentStateActor::new();
        actor.set_error(Some("previous failure".into())).await;
        actor.apply_event(&AgentEvent::AgentStart).await;
        actor.sync().await;
        let snapshot = actor.snapshot();
        assert!(snapshot.is_streaming);
        assert!(snapshot.streaming_message.is_none());
        assert!(snapshot.pending_tool_calls.is_empty());
        assert!(snapshot.error_message.is_none());
    }

    #[tokio::test]
    async fn agent_end_clears_interrupted_pending_tool_calls() {
        let actor = AgentStateActor::new();
        actor.add_pending_tool_call("call-1".into()).await;
        actor
            .apply_event(&AgentEvent::AgentEnd { messages: vec![] })
            .await;
        actor.sync().await;
        assert!(actor.snapshot().pending_tool_calls.is_empty());
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
