//! Event Queue (EQ) types: Core → TUI.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::op::{ApprovalId, SubmissionId};

/// Error code carried by event-queue error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Internal error.
    Internal,
    /// Submission was invalid.
    InvalidSubmission,
    /// Tool execution was rejected.
    ToolRejected,
    /// Session not found.
    SessionNotFound,
}

/// Event message sent from the core to the TUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "msg", rename_all = "snake_case")]
pub enum EventMsg {
    /// A turn started.
    TurnStarted {
        /// Turn id.
        turn_id: u64,
    },
    /// A turn completed.
    TurnComplete {
        /// Turn id.
        turn_id: u64,
        /// Response id.
        response_id: String,
    },
    /// An agent message chunk.
    AgentMessage {
        /// Message content.
        content: String,
    },
    /// Request approval for a tool execution.
    ExecApprovalRequest {
        /// Approval request id.
        id: ApprovalId,
        /// Tool name.
        tool: String,
        /// Tool arguments.
        args: Value,
    },
    /// Error event.
    Error {
        /// Error code.
        code: ErrorCode,
        /// Error message.
        message: String,
    },
}

/// An event on the Event Queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Correlated submission id, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SubmissionId>,
    /// Event payload.
    pub msg: EventMsg,
}

impl Event {
    /// Create a new event with no correlation.
    pub fn new(msg: EventMsg) -> Self {
        Self { id: None, msg }
    }

    /// Create a new event correlated to a submission.
    pub fn correlated(id: SubmissionId, msg: EventMsg) -> Self {
        Self {
            id: Some(id),
            msg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_variant_roundtrip() {
        let variants = vec![
            EventMsg::TurnStarted { turn_id: 1 },
            EventMsg::TurnComplete {
                turn_id: 2,
                response_id: "r1".into(),
            },
            EventMsg::AgentMessage {
                content: "hi".into(),
            },
            EventMsg::ExecApprovalRequest {
                id: ApprovalId::new("a1"),
                tool: "read".into(),
                args: serde_json::json!({"path": "/tmp"}),
            },
            EventMsg::Error {
                code: ErrorCode::Internal,
                message: "boom".into(),
            },
        ];
        for msg in variants {
            let json = serde_json::to_string(&msg).unwrap();
            let parsed: EventMsg = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, msg);
        }
    }
}
