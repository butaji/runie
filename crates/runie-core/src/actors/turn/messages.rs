//! TurnActor message types.
//!
//! These messages control the turn lifecycle: queuing, starting, aborting,
//! and tracking progress through an agent turn.

use ractor::RpcReplyPort;
use serde::{Deserialize, Serialize};

/// Messages accepted by TurnActor.
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Whether a submitted message came from a fresh user submit or a queued/delivered source.
#[derive(Copy, PartialEq, Eq, Default)]
pub enum MessageSource {
    /// Fresh user submit — emit UserMessageSubmitted.
    #[default]
    Fresh,
    /// Queued/delivered content — content already in session via FollowUpDelivered;
    /// do NOT emit UserMessageSubmitted again.
    Queued,
}

/// Messages accepted by TurnActor.
// Clone is manual (RpcReplyPort is not Clone); Serialize/Deserialize removed
// (TurnMsg is in-memory actor messaging only, not persisted/cross-process).
#[derive(Debug)]
pub enum TurnMsg {
    /// Check queue and start a turn if something is queued.
    RunIfQueued,
    /// Abort the current turn and stop the queue.
    AbortTurn,
    /// Submit a user message to the queue.
    /// `source` indicates whether this is a fresh submit (should emit UserMessageSubmitted)
    /// or a queued/delivered message (content already in session via FollowUpDelivered).
    SubmitUserMessage { content: String, id: String, source: MessageSource },
    /// Queue a steering message.
    QueueSteering { content: String },
    /// Queue a follow-up message.
    QueueFollowUp { content: String },
    /// Abort the message queue (move messages back to input).
    AbortQueue,
    /// Clear all queues.
    ClearQueues,
    /// Deliver queued messages to the request queue.
    DeliverQueued {
        steering_mode: crate::model::DeliveryMode,
        follow_up_mode: crate::model::DeliveryMode,
        /// Optional RPC reply port — `Some(port)` for RPC callers that wait for a reply;
        /// `None` for fire-and-forget callers. The actor sends a reply only when `Some`.
        reply: Option<RpcReplyPort<Option<DeliverQueuedResponse>>>,
    },
    /// Dequeue the last message back to input.
    Dequeue,
    /// LLM event: thinking started.
    Thinking { id: String },
    /// LLM event: thought done.
    ThoughtDone { id: String },
    /// LLM event: tool started.
    ToolStart { id: String, name: String },
    /// LLM event: tool ended.
    ToolEnd {
        id: String,
        duration_secs: f64,
        output: String,
    },
    /// LLM event: response delta.
    ResponseDelta { id: String, content: String },
    /// LLM event: turn complete.
    TurnComplete { id: String, duration_secs: f64 },
    /// LLM event: done.
    Done { id: String },
    /// LLM event: error.
    Error { id: String, message: String },
    /// Update speed stats.
    UpdateSpeed { tokens_out: usize },
    /// Generate next message ID.
    NextId,
}

impl Clone for TurnMsg {
    fn clone(&self) -> Self {
        match self {
            TurnMsg::RunIfQueued => TurnMsg::RunIfQueued,
            TurnMsg::AbortTurn => TurnMsg::AbortTurn,
            TurnMsg::SubmitUserMessage { content, id, source } => TurnMsg::SubmitUserMessage {
                content: content.clone(),
                id: id.clone(),
                source: *source,
            },
            TurnMsg::QueueSteering { content } => TurnMsg::QueueSteering { content: content.clone() },
            TurnMsg::QueueFollowUp { content } => TurnMsg::QueueFollowUp { content: content.clone() },
            TurnMsg::AbortQueue => TurnMsg::AbortQueue,
            TurnMsg::ClearQueues => TurnMsg::ClearQueues,
            TurnMsg::DeliverQueued { steering_mode, follow_up_mode, .. } => TurnMsg::DeliverQueued {
                steering_mode: *steering_mode,
                follow_up_mode: *follow_up_mode,
                reply: None, // Fire-and-forget; the original reply is not cloned.
            },
            TurnMsg::Dequeue => TurnMsg::Dequeue,
            TurnMsg::Thinking { id } => TurnMsg::Thinking { id: id.clone() },
            TurnMsg::ThoughtDone { id } => TurnMsg::ThoughtDone { id: id.clone() },
            TurnMsg::ToolStart { id, name } => TurnMsg::ToolStart { id: id.clone(), name: name.clone() },
            TurnMsg::ToolEnd { id, duration_secs, output } => TurnMsg::ToolEnd {
                id: id.clone(),
                duration_secs: *duration_secs,
                output: output.clone(),
            },
            TurnMsg::ResponseDelta { id, content } => TurnMsg::ResponseDelta {
                id: id.clone(),
                content: content.clone(),
            },
            TurnMsg::TurnComplete { id, duration_secs } => TurnMsg::TurnComplete {
                id: id.clone(),
                duration_secs: *duration_secs,
            },
            TurnMsg::Done { id } => TurnMsg::Done { id: id.clone() },
            TurnMsg::Error { id, message } => TurnMsg::Error { id: id.clone(), message: message.clone() },
            TurnMsg::UpdateSpeed { tokens_out } => TurnMsg::UpdateSpeed { tokens_out: *tokens_out },
            TurnMsg::NextId => TurnMsg::NextId,
        }
    }
}

/// Response type for NextId.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NextIdResponse {
    pub id: String,
}

/// What the TurnActor delivered when processing `DeliverQueued`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DeliverQueuedResponse {
    /// A steering message was moved from message_queue to request_queue.
    Steering { content: String, id: String },
    /// A follow-up message was moved from message_queue to request_queue.
    FollowUp { content: String, id: String },
    /// Nothing was in the queue to deliver.
    None,
}

/// Result of the DeliverQueued RPC call, mapping ractor's CallResult.
/// Exposed so callers (TUI) don't need a direct `ractor` dependency.
#[derive(Debug)]
pub enum DeliverQueuedRpcResult {
    /// Successfully delivered (steering, follow-up, or nothing).
    Delivered(Option<DeliverQueuedResponse>),
    /// The reply channel was closed before a reply was sent.
    SenderError,
    /// The actor returned an error.
    ActorError(String),
}
