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
        if apply_workflow_event(state, &event) {
            return;
        }
        if apply_background_event(state, &event) {
            return;
        }
        if apply_message_tool_event(state, &event) {
            return;
        }
        if apply_core_state_event(state, &event) {
            return;
        }
        let _ = Self::apply_unowned_event(&event);
    }

    /// Keep events owned by other actors explicit at this projection boundary.
    fn apply_unowned_event(_: &AgentEvent) -> bool {
        true
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

fn apply_workflow_event(state: &mut AgentStateSnapshot, event: &AgentEvent) -> bool {
    match event {
        AgentEvent::WorkflowStarted {
            run_id,
            name,
            objective,
        } => start_workflow(state, run_id, name, objective),
        AgentEvent::WorkflowProgress {
            run_id,
            phase,
            state: phase_state,
            active_agents,
        } => update_workflow(state, run_id, phase, phase_state, *active_agents),
        AgentEvent::WorkflowFinished {
            run_id,
            status,
            elapsed_ms,
        } => finish_workflow(state, run_id, status, *elapsed_ms),
        _ => return false,
    }
    true
}

fn start_workflow(state: &mut AgentStateSnapshot, run_id: &str, name: &str, objective: &str) {
    state.workflows.insert(
        run_id.to_owned(),
        WorkflowSnapshot {
            name: name.to_owned(),
            objective: objective.to_owned(),
            status: "active".into(),
            ..WorkflowSnapshot::default()
        },
    );
}

fn update_workflow(
    state: &mut AgentStateSnapshot,
    run_id: &str,
    phase: &str,
    phase_state: &str,
    active_agents: u32,
) {
    if let Some(workflow) = state.workflows.get_mut(run_id) {
        workflow.phase = Some(phase.to_owned());
        workflow.state = Some(phase_state.to_owned());
        workflow.active_agents = active_agents;
    }
}

fn finish_workflow(
    state: &mut AgentStateSnapshot,
    run_id: &str,
    status: &str,
    elapsed_ms: Option<u64>,
) {
    if let Some(workflow) = state.workflows.get_mut(run_id) {
        workflow.status = status.to_owned();
        workflow.elapsed_ms = elapsed_ms;
    }
}

fn apply_background_event(state: &mut AgentStateSnapshot, event: &AgentEvent) -> bool {
    match event {
        AgentEvent::BackgroundWorkStarted {
            work_id,
            description,
            background,
        } => start_background_work(state, work_id, description, *background),
        AgentEvent::BackgroundWorkProgress {
            work_id,
            description,
            activity,
        } => update_background_work(state, work_id, description, activity),
        AgentEvent::BackgroundWorkFinished {
            work_id,
            description,
            is_error,
            elapsed_ms,
            error,
        } => finish_background_work(state, work_id, description, *is_error, *elapsed_ms, error),
        AgentEvent::BackgroundWorkCancelled {
            work_id,
            description,
            elapsed_ms,
        } => cancel_background_work(state, work_id, description, *elapsed_ms),
        _ => return false,
    }
    true
}

fn start_background_work(
    state: &mut AgentStateSnapshot,
    work_id: &str,
    description: &str,
    background: bool,
) {
    state.background_work.insert(
        work_id.to_owned(),
        BackgroundWorkSnapshot {
            description: description.to_owned(),
            background,
            status: "running".into(),
            ..BackgroundWorkSnapshot::default()
        },
    );
}

fn update_background_work(
    state: &mut AgentStateSnapshot,
    work_id: &str,
    description: &str,
    activity: &str,
) {
    let work = state.background_work.entry(work_id.to_owned()).or_default();
    work.description = description.to_owned();
    work.activity = Some(activity.to_owned());
    work.status = "running".into();
}

fn finish_background_work(
    state: &mut AgentStateSnapshot,
    work_id: &str,
    description: &str,
    is_error: bool,
    elapsed_ms: Option<u64>,
    error: &Option<String>,
) {
    let work = state.background_work.entry(work_id.to_owned()).or_default();
    work.description = description.to_owned();
    work.status = if is_error { "failed" } else { "done" }.into();
    work.elapsed_ms = elapsed_ms;
    work.error = error.clone();
}

fn cancel_background_work(
    state: &mut AgentStateSnapshot,
    work_id: &str,
    description: &str,
    elapsed_ms: Option<u64>,
) {
    let work = state.background_work.entry(work_id.to_owned()).or_default();
    work.description = description.to_owned();
    work.status = "cancelled".into();
    work.elapsed_ms = elapsed_ms;
}

fn apply_message_tool_event(state: &mut AgentStateSnapshot, event: &AgentEvent) -> bool {
    match event {
        AgentEvent::MessageStart { message } if is_assistant(message) => {
            state.is_streaming = true;
            state.streaming_message = Some(message.clone());
        }
        AgentEvent::MessageUpdate { message, .. } => {
            state.streaming_message = Some(message.clone());
        }
        AgentEvent::MessageEnd { message } => {
            state.messages.push(message.clone());
            if matches!(message, AgentMessage::Assistant(_)) {
                state.streaming_message = None;
            }
        }
        AgentEvent::ToolExecutionStart { tool_call_id, .. } => {
            if !state.pending_tool_calls.contains(tool_call_id) {
                state.pending_tool_calls.push(tool_call_id.clone());
            }
        }
        AgentEvent::ToolExecutionEnd { tool_call_id, .. } => {
            state.pending_tool_calls.retain(|id| id != tool_call_id);
        }
        _ => return false,
    }
    true
}

fn apply_core_state_event(state: &mut AgentStateSnapshot, event: &AgentEvent) -> bool {
    match event {
        AgentEvent::AgentStart => {
            state.is_streaming = true;
            state.streaming_message = None;
            state.pending_tool_calls.clear();
            state.error_message = None;
        }
        AgentEvent::ModelChanged { model } => state.model = model.clone(),
        AgentEvent::Error { message } => {
            state.is_streaming = false;
            state.streaming_message = None;
            state.error_message = Some(message.clone());
        }
        AgentEvent::ThinkingLevelChanged { level } => state.thinking_level = *level,
        AgentEvent::TurnEnd { message, .. } => {
            if let AgentMessage::Assistant(assistant) = message {
                if assistant.error_message.is_some() {
                    state.error_message = assistant.error_message.clone();
                }
            }
        }
        AgentEvent::Reset => *state = AgentStateSnapshot::default(),
        AgentEvent::AgentEnd { .. } => {
            state.is_streaming = false;
            state.streaming_message = None;
            state.pending_tool_calls.clear();
        }
        _ => return false,
    }
    true
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

#[path = "actor_commands.rs"]
mod actor_commands;
use actor_commands::apply;

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;
