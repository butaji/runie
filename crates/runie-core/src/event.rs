use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{ToolOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Event {
    AgentStart { session_id: String, timestamp: DateTime<Utc> },
    TurnStart { turn: usize, timestamp: DateTime<Utc> },
    MessageStart { role: String, timestamp: DateTime<Utc> },
    MessageDelta { content: String },
    ThinkingDelta { content: String },
    ToolCallDelta { name: String, arguments: String },
    MessageEnd,
    ToolExecutionStart { tool_call_id: String, tool_name: String, args: serde_json::Value, timestamp: DateTime<Utc> },
    ToolExecutionEnd { tool_call_id: String, result: ToolOutput, timestamp: DateTime<Utc> },
    ToolExecutionError { tool_call_id: String, error: String },
    Compaction { summary: String, original_turns: usize, compacted_turns: usize },
    Error { message: String },
    AgentEnd { timestamp: DateTime<Utc> },
}
