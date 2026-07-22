//! ACP message types.
//!
//! Defines the message envelope and all message variants for leader/client communication.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Client to server message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Register a new client connection.
    Register {
        client_type: String,
        mode: ClientMode,
        capabilities: ClientCapabilities,
    },
    /// ACP payload forwarded to the agent.
    Acp {
        session_id: Uuid,
        payload: String,
    },
    /// Control command (info, profile, workspace control).
    Control {
        request_id: String,
        command: ControlCommand,
    },
    /// Ping for keepalive.
    Ping,
}

/// Server to client message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Registration response from leader.
    Registered {
        client_id: u64,
        ready: bool,
        leader_protocol_version: Option<u32>,
        leader_binary_version: Option<String>,
    },
    /// ACP payload from the agent to clients.
    Acp {
        session_id: Uuid,
        payload: String,
    },
    /// Control command result.
    ControlResult {
        request_id: String,
        result: Result<(), ControlError>,
    },
    /// Session update (streaming, subagent events, etc.).
    SessionUpdate {
        session_id: Uuid,
        update: SessionUpdate,
    },
    /// Pong response to keepalive.
    Pong,
    /// Error from server.
    Error {
        code: i32,
        message: String,
    },
}

/// Client connection mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientMode {
    /// Headless mode (websocket).
    Headless,
    /// Local stdio mode.
    Stdio,
}

/// Client capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    pub yolo_mode: bool,
    pub auto_mode: bool,
    pub default_model: Option<String>,
    pub client_version: Option<String>,
    pub code_nav_enabled: bool,
    pub terminal: bool,
    pub fs_read: bool,
    pub fs_write: bool,
}

/// Control commands from client to leader.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ControlCommand {
    /// Get leader info.
    GetLeaderInfo,
    /// Start CPU profiling.
    StartCpuProfile,
    /// Stop CPU profiling.
    StopCpuProfile,
    /// CPU profile status.
    CpuProfileStatus,
    /// Workspace operations.
    Workspace {
        action: WorkspaceAction,
    },
}

/// Workspace control actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum WorkspaceAction {
    Start,
    Pause,
    Resume,
    Stop,
    Status,
}

/// Control error variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlError {
    NotFound,
    InvalidState,
    InternalError,
    Unauthorized,
}

/// Session update types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionUpdate {
    /// Streaming text delta.
    TextDelta {
        content: String,
    },
    /// Subagent spawned.
    SubagentSpawned {
        subagent_id: Uuid,
        description: String,
    },
    /// Subagent finished.
    SubagentFinished {
        subagent_id: Uuid,
        success: bool,
        output: String,
        error: Option<String>,
    },
    /// Tool call started.
    ToolStart {
        name: String,
        input: serde_json::Value,
    },
    /// Tool call finished.
    ToolEnd {
        name: String,
        output: String,
    },
    /// Error occurred.
    Error {
        message: String,
    },
    /// Done signal.
    Done,
}

/// Raw ACP message wrapper (JSON-RPC style).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpMessage {
    pub id: Option<Uuid>,
    pub method: String,
    pub params: serde_json::Value,
}

impl AcpMessage {
    /// Create a new ACP message.
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            id: Some(Uuid::new_v4()),
            method: method.into(),
            params,
        }
    }
}
