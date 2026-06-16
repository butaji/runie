//! Durable event types for JSONL session persistence.
//!
//! These events are appended to `data_dir/runie/sessions/<id>.jsonl` and
//! can be replayed to reconstruct a session.

use serde::{Deserialize, Serialize};

/// Events that are persisted to the session JSONL log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum DurableCoreEvent {
    /// A message sent by the user or the assistant.
    MessageSent {
        id: String,
        role: String,
        content: String,
        timestamp: f64,
    },
    /// An LLM invoked a tool.
    ToolCalled {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// A tool returned its result.
    ToolResult {
        id: String,
        output: String,
        success: bool,
    },
    /// The user switched the active model or provider.
    ModelSwitched {
        provider: String,
        model: String,
    },
    /// The session was renamed by the user.
    SessionRenamed {
        name: String,
    },
}
