//! Agent event variants (LLM responses, tool calls, turn lifecycle).

use std::fmt;
use strum::IntoStaticStr;

/// LLM response and tool call events emitted by the agent.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum AgentEvent {
    /// Agent started thinking.
    Thinking { id: String },
    /// Agent finished thinking and is ready to act.
    ThoughtDone { id: String },
    /// Agent started calling a tool.
    ToolStart {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Agent finished tool execution.
    ToolEnd {
        id: String,
        duration_secs: f64,
        output: String,
    },
    /// Transient streaming delta — NOT persisted to session.
    ResponseDelta { id: String, content: String },
    /// Complete response — persisted to session as MessageSent.
    Response { id: String, content: String },
    /// Agent completed a full turn.
    TurnComplete { id: String, duration_secs: f64 },
    /// Agent finished (all turns done or halted).
    Done { id: String },
    /// Agent encountered an error.
    Error { id: String, message: String },
}

impl fmt::Display for AgentEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentEvent::Thinking { .. } => write!(f, "Thinking"),
            AgentEvent::ThoughtDone { .. } => write!(f, "ThoughtDone"),
            AgentEvent::ToolStart { .. } => write!(f, "ToolStart"),
            AgentEvent::ToolEnd { .. } => write!(f, "ToolEnd"),
            AgentEvent::ResponseDelta { .. } => write!(f, "ResponseDelta"),
            AgentEvent::Response { .. } => write!(f, "Response"),
            AgentEvent::TurnComplete { .. } => write!(f, "TurnComplete"),
            AgentEvent::Done { .. } => write!(f, "Done"),
            AgentEvent::Error { .. } => write!(f, "Error"),
        }
    }
}
