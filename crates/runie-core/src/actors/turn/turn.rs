//! `TurnActor` — owns agent turn lifecycle and queues.
//!
//! No ractor dependency. Actor is a tokio task with mpsc channel.

use tokio::sync::mpsc;
use tracing::instrument;

use crate::bus::EventBus;
use crate::Event;

use super::handlers;
use super::messages::TurnMsg;
use super::state::TurnActorState;
use super::messages::DeliverQueuedRpcResult;

/// TurnActor handle — cloneable, fire-and-forget sender.
#[derive(Clone, Debug)]
pub struct TurnHandle {
    tx: mpsc::UnboundedSender<TurnMsg>,
}

impl TurnHandle {
    /// Create a new handle wrapping a sender.
    pub fn new(tx: mpsc::UnboundedSender<TurnMsg>) -> Self {
        Self { tx }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: TurnMsg) {
        let _ = self.tx.send(msg);
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: TurnMsg) -> Result<(), mpsc::error::SendError<TurnMsg>> {
        self.tx.send(msg)
    }

    /// Deliver queued messages and wait for the actor to emit SteeringDelivered/
    /// FollowUpDelivered before returning.
    pub async fn deliver_queued(
        &self,
        steering_mode: crate::model::DeliveryMode,
        follow_up_mode: crate::model::DeliveryMode,
    ) -> DeliverQueuedRpcResult {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = TurnMsg::DeliverQueued { steering_mode, follow_up_mode, reply: Some(tx) };
        let _ = self.tx.send(msg);
        match rx.await {
            Ok(r) => DeliverQueuedRpcResult::Delivered(r),
            Err(_) => DeliverQueuedRpcResult::SenderError,
        }
    }
}

// Backward-compat aliases
#[allow(unused_imports)]
pub use TurnHandle as RactorTurnHandle;

// ── Actor struct ────────────────────────────────────────────────────────────

/// The TurnActor — processes turn lifecycle messages.
struct TurnActor {
    rx: mpsc::UnboundedReceiver<TurnMsg>,
    state: TurnActorState,
}

impl TurnActor {
    /// Main loop.
    async fn run(&mut self) {
        while let Some(msg) = self.rx.recv().await {
            self.handle(msg).await;
        }
    }

    /// Handle one message.
    #[instrument(name = "turn_actor", skip_all, fields(msg = ?msg))]
    async fn handle(&mut self, msg: TurnMsg) {
        match msg {
            TurnMsg::RunIfQueued => handlers::handle_run_if_queued(&mut self.state),
            TurnMsg::AbortTurn => handlers::handle_abort_turn(&mut self.state),
            TurnMsg::SubmitUserMessage { content, id, source } => {
                handlers::handle_submit_user_message(&mut self.state, content, id, source)
            }
            TurnMsg::QueueSteering { content } => handlers::handle_queue_steering(&mut self.state, content),
            TurnMsg::QueueFollowUp { content } => handlers::handle_queue_follow_up(&mut self.state, content),
            TurnMsg::AbortQueue => handlers::handle_abort_queue(&mut self.state),
            TurnMsg::ClearQueues => handlers::handle_clear_queues(&mut self.state),
            TurnMsg::DeliverQueued { steering_mode, follow_up_mode, reply } => {
                handlers::handle_deliver_queued(&mut self.state, steering_mode, follow_up_mode, reply)
            }
            TurnMsg::Dequeue => handlers::handle_dequeue(&mut self.state),
            TurnMsg::Thinking { id } => handlers::handle_thinking(&mut self.state, id),
            TurnMsg::ThoughtDone { id: _ } => handlers::handle_thought_done(&mut self.state),
            TurnMsg::ToolStart { id, name } => handlers::handle_tool_start(&mut self.state, id, name),
            TurnMsg::ToolEnd { id, duration_secs, output } => {
                handlers::handle_tool_end(&mut self.state, id, duration_secs, output)
            }
            TurnMsg::ResponseDelta { id, content } => handlers::handle_response_delta(&mut self.state, id, content),
            TurnMsg::TurnComplete { id, duration_secs } => {
                handlers::handle_turn_complete(&mut self.state, id, duration_secs)
            }
            TurnMsg::Done { id: _ } => handlers::handle_done(&mut self.state),
            TurnMsg::Error { id, message } => handlers::handle_error(&mut self.state, id, message),
            TurnMsg::UpdateSpeed { tokens_out } => handlers::handle_update_speed(&mut self.state, tokens_out),
            TurnMsg::NextId => handlers::handle_next_id(&mut self.state),
            TurnMsg::ConfigureTokenTracker { provider, model } => {
                handlers::handle_configure_token_tracker(&mut self.state, provider, model)
            }
        }
    }
}

// ── Spawn ───────────────────────────────────────────────────────────────────

/// Spawn a TurnActor and return (handle, stop_cell, join_handle).
pub fn spawn_turn_actor(bus: EventBus<Event>) -> (TurnHandle, crate::actors::StopCell, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let state = TurnActorState::new(bus);
    let mut actor = TurnActor { rx, state };

    let join = tokio::spawn(async move {
        actor.run().await;
    });

    (TurnHandle::new(tx), crate::actors::StopCell, join)
}

/// Stub type for backward compatibility.
pub struct TurnActorBase;

/// Backward-compat alias.
#[allow(dead_code)]
pub type RactorTurnActor = TurnActorBase;

impl TurnActorBase {
    /// Spawn a `TurnActor` and return (handle, cell, join).
    /// Cell is a no-op `StopCell` — mpsc actors stop when the handle is dropped.
    pub async fn spawn(
        bus: EventBus<Event>,
    ) -> Result<(TurnHandle, crate::actors::StopCell, tokio::task::JoinHandle<()>), anyhow::Error> {
        Ok(spawn_turn_actor(bus))
    }
}
