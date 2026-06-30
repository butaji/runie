//! TurnActor message types.
//!
//! These messages control the turn lifecycle: queuing, starting, aborting,
//! and tracking progress through an agent turn.

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Response type for NextId.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NextIdResponse {
    pub id: String,
}
