//! Durable event types for session persistence.
//!
//! These events are stored in `SessionStore` (redb) under
//! `data_dir/runie/sessions/<id>.redb` and can be replayed to reconstruct a
//! session.

use serde::{Deserialize, Serialize};

/// Events that are persisted to the session store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum DurableCoreEvent {
    /// A message sent by the user or the assistant.
    MessageSent {
        id: String,
        role: String,
        content: String,
        timestamp: f64,
        #[serde(default)]
        provider: String,
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
    ModelSwitched { provider: String, model: String },
    /// The session was renamed by the user.
    SessionRenamed { name: String },
    /// The user switched the active theme.
    ThemeSwitched { name: String },
    /// The user changed the thinking level.
    ThinkingLevelSet { level: crate::model::ThinkingLevel },
    /// The user toggled read-only mode.
    ReadOnlySet { read_only: bool },
}
