//! Event queue types for core → TUI IPC.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::op::{ApprovalId, SubmissionId};

/// W3C trace context propagated across the queue pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct W3cTraceContext {
    pub traceparent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracestate: Option<String>,
}

/// Typed error codes emitted by the core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Internal,
    InvalidSubmission,
    Disconnected,
    UserDenied,
}

/// A notification from core to TUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "msg", rename_all = "snake_case")]
pub enum EventMsg {
    TurnStarted {
        turn_id: u64,
    },
    TurnComplete {
        turn_id: u64,
        response_id: String,
    },
    AgentMessage {
        content: String,
    },
    ExecApprovalRequest {
        id: ApprovalId,
        tool: String,
        args: Value,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
}

/// Event on the core → TUI queue, optionally correlated to a submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<SubmissionId>,
    pub msg: EventMsg,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op::{
        ApprovalDecision, ApprovalId, Op, PromptOrigin, SessionConfig, Submission, SubmissionId,
    };

    fn roundtrip<T>(value: &T) -> T
    where
        T: Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        serde_json::from_value(serde_json::to_value(value).unwrap()).unwrap()
    }

    #[test]
    fn event_variant_roundtrip() {
        let events = vec![
            EventMsg::TurnStarted { turn_id: 1 },
            EventMsg::TurnComplete {
                turn_id: 2,
                response_id: "r3".into(),
            },
            EventMsg::AgentMessage {
                content: "hi".into(),
            },
            EventMsg::ExecApprovalRequest {
                id: ApprovalId(9),
                tool: "write".into(),
                args: serde_json::json!({"path": "/tmp/x"}),
            },
            EventMsg::Error {
                code: ErrorCode::Internal,
                message: "oops".into(),
            },
        ];
        for msg in events {
            assert_eq!(roundtrip(&msg), msg);
        }
    }

    #[test]
    fn submission_id_correlates_event() {
        let id = SubmissionId(42);
        let submission = Submission {
            id,
            op: Op::Interrupt,
            trace: None,
        };
        let event = Event {
            id: Some(submission.id),
            msg: EventMsg::TurnStarted { turn_id: 1 },
        };
        assert_eq!(event.id, Some(submission.id));
    }
}
