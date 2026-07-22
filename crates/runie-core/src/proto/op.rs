//! Submission Queue (SQ) types: TUI → Core.

use serde::{Deserialize, Serialize};

/// Unique identifier for a submission on the SQ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubmissionId(pub u64);

impl SubmissionId {
    /// Create a new submission id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Origin of a user prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptOrigin {
    /// Typed directly by the user.
    User,
    /// Auto-continued by the system.
    Continuation,
}

/// Decision for an execution approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Allow the tool execution.
    Allow,
    /// Deny the tool execution.
    Deny,
}

/// Identifier for an approval request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApprovalId(pub String);

impl ApprovalId {
    /// Create a new approval id.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Session configuration payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Optional model override.
    pub model: Option<String>,
}

impl SessionConfig {
    /// Create a session config with a model override.
    pub fn new(model: impl Into<String>) -> Self {
        Self { model: Some(model.into()) }
    }
}

/// W3C trace context propagated with a submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct W3cTraceContext {
    /// W3C traceparent header value.
    pub trace_parent: String,
    /// W3C tracestate header value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_state: Option<String>,
}

impl W3cTraceContext {
    /// Create a trace context.
    pub fn new(trace_parent: impl Into<String>) -> Self {
        Self { trace_parent: trace_parent.into(), trace_state: None }
    }

    /// Attach a tracestate value.
    pub fn with_trace_state(mut self, trace_state: impl Into<String>) -> Self {
        self.trace_state = Some(trace_state.into());
        self
    }
}

/// Operation sent from the TUI to the core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// User turn with input and origin.
    UserTurn {
        /// Input text.
        input: String,
        /// Origin of the prompt.
        origin: PromptOrigin,
    },
    /// Interrupt the current turn.
    Interrupt,
    /// Respond to an execution approval request.
    ExecApproval {
        /// Approval request id.
        id: ApprovalId,
        /// Decision.
        decision: ApprovalDecision,
    },
    /// Answer a user-input question.
    UserInputAnswer {
        /// Question id.
        question_id: String,
        /// Answer text.
        answer: String,
    },
    /// Configure the session.
    ConfigureSession {
        /// Session configuration.
        config: SessionConfig,
    },
    /// Shut down the core.
    Shutdown,
}

/// A submission on the Submission Queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submission {
    /// Submission id.
    pub id: SubmissionId,
    /// Operation to perform.
    pub op: Op,
    /// Optional W3C trace context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<W3cTraceContext>,
}

impl Submission {
    /// Create a new submission.
    pub fn new(id: SubmissionId, op: Op) -> Self {
        Self { id, op, trace: None }
    }

    /// Attach a trace context.
    pub fn with_trace(mut self, trace: W3cTraceContext) -> Self {
        self.trace = Some(trace);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_variant_roundtrip() {
        let variants = vec![
            Op::UserTurn { input: "hello".into(), origin: PromptOrigin::User },
            Op::Interrupt,
            Op::ExecApproval { id: ApprovalId::new("a1"), decision: ApprovalDecision::Allow },
            Op::UserInputAnswer { question_id: "q1".into(), answer: "yes".into() },
            Op::ConfigureSession { config: SessionConfig::new("gpt-4") },
            Op::Shutdown,
        ];
        for op in variants {
            let json = serde_json::to_string(&op).unwrap();
            let parsed: Op = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, op);
        }
    }

    #[test]
    fn submission_with_trace_roundtrips() {
        let trace = W3cTraceContext::new("00-abc-def-01").with_trace_state("vendor=kimi");
        let sub = Submission::new(SubmissionId::new(7), Op::Interrupt).with_trace(trace);
        let json = serde_json::to_string(&sub).unwrap();
        let parsed: Submission = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, sub);
    }
}
