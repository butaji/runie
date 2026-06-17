//! Submission queue types for TUI → core IPC.

use serde::{Deserialize, Serialize};

/// Opaque identifier for a submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubmissionId(pub u64);

/// Origin of a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptOrigin {
    UserInput,
    Remote,
    Replay,
}

/// Identifier for an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApprovalId(pub u64);

/// Decision on an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Reject,
}

/// Session configuration carried with a configure op.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionConfig {
    pub model: String,
    pub system_prompt: Option<String>,
}

/// A single submission on the TUI → core queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submission {
    pub id: SubmissionId,
    pub op: Op,
    pub trace: Option<crate::event::W3cTraceContext>,
}

/// Operations the TUI can submit to the core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    UserTurn {
        input: String,
        origin: PromptOrigin,
    },
    Interrupt,
    ExecApproval {
        id: ApprovalId,
        decision: ApprovalDecision,
    },
    UserInputAnswer {
        question_id: String,
        answer: String,
    },
    ConfigureSession {
        config: SessionConfig,
    },
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip<T>(value: &T) -> T
    where
        T: Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        serde_json::from_value(serde_json::to_value(value).unwrap()).unwrap()
    }

    #[test]
    fn op_variant_roundtrip() {
        let ops = vec![
            Op::UserTurn {
                input: "hello".into(),
                origin: PromptOrigin::UserInput,
            },
            Op::Interrupt,
            Op::ExecApproval {
                id: ApprovalId(7),
                decision: ApprovalDecision::Approve,
            },
            Op::UserInputAnswer {
                question_id: "q1".into(),
                answer: "yes".into(),
            },
            Op::ConfigureSession {
                config: SessionConfig {
                    model: "gpt-4".into(),
                    system_prompt: Some("be helpful".into()),
                },
            },
            Op::Shutdown,
        ];
        for op in ops {
            assert_eq!(roundtrip(&op), op);
        }
    }
}
