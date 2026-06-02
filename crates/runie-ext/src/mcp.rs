//! MCP (Model Context Protocol) server connections
//!
//! MCP servers are external processes that communicate with runie via
//! JSON-RPC over stdio. This module handles spawning, managing, and
//! communicating with these servers.
//!
//! ## Transport Options
//!
//! - **stdio**: Default - JSON-RPC over stdin/stdout (subprocess)
//! - **TCP**: Optional - JSON-RPC over TCP socket (for long-running servers)
//!
//! ## Protocol
//!
//! MCP uses JSON-RPC 2.0 with the following base protocol:
//!
//! ```json
//! // Request
//! {"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}
//!
//! // Response
//! {"jsonrpc": "2.0", "id": 1, "result": {...}}
//! ```

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{timeout, Duration};

/// MCP server connection manager
pub struct McpRegistry {
    servers: RwLock<HashMap<String, Arc<McpServerHandle>>>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
        }
    }

    /// Spawn and connect to an MCP server
    pub async fn connect(&self, config: McpServerConfig) -> Result<String, McpError> {
        let handle = McpServerHandle::spawn(config).await?;
        let id = handle.id.clone();
        let mut servers = self.servers.write().await;
        servers.insert(id.clone(), Arc::new(handle));
        tracing::info!("Connected to MCP server: {}", id);
        Ok(id)
    }

    /// Disconnect from an MCP server
    pub async fn disconnect(&self, id: &str) -> Result<(), McpError> {
        let mut servers = self.servers.write().await;
        if let Some(handle) = servers.remove(id) {
            handle.kill().await?;
            tracing::info!("Disconnected from MCP server: {}", id);
            Ok(())
        } else {
            Err(McpError::NotFound(id.to_string()))
        }
    }

    /// List connected servers
    pub async fn list(&self) -> Vec<McpServerInfo> {
        let servers = self.servers.read().await;
        let handles: Vec<_> = servers.values().collect();
        let mut infos = Vec::new();
        for h in handles {
            let connected = h.is_connected().await;
            infos.push(McpServerInfo {
                id: h.id.clone(),
                name: h.name.clone(),
                transport: h.transport.clone(),
                status: if connected { "connected" } else { "disconnected" }.to_string(),
            });
        }
        infos
    }

    /// Call a tool on an MCP server
    pub async fn call_tool(&self, server_id: &str, tool_name: &str, args: serde_json::Value) -> Result<McpToolResult, McpError> {
        let servers = self.servers.read().await;
        let handle = servers.get(server_id)
            .ok_or_else(|| McpError::NotFound(server_id.to_string()))?;
        handle.call_tool(tool_name, args).await
    }

    /// List tools available on an MCP server
    pub async fn list_tools(&self, server_id: &str) -> Result<Vec<McpTool>, McpError> {
        let servers = self.servers.read().await;
        let handle = servers.get(server_id)
            .ok_or_else(|| McpError::NotFound(server_id.to_string()))?;
        handle.list_tools().await
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP server configuration
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub transport: McpTransport,
}

impl McpServerConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            transport: McpTransport::Stdio,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }
}

/// Transport type for MCP connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum McpTransport {
    /// JSON-RPC over stdin/stdout (subprocess)
    Stdio,
    /// JSON-RPC over TCP socket
    Tcp,
}

/// Active MCP server handle
struct McpServerHandle {
    id: String,
    name: String,
    transport: McpTransport,
    child: Mutex<Option<Child>>,
    stdin: mpsc::Sender<McpRequest>,
    stdout: mpsc::Receiver<McpResponse>,
    connected: RwLock<bool>,
}

impl McpServerHandle {
    /// Spawn a new MCP server process
    async fn spawn(config: McpServerConfig) -> Result<Self, McpError> {
        match config.transport {
            McpTransport::Stdio => Self::spawn_stdio(config).await,
            McpTransport::Tcp => Self::spawn_tcp(config).await,
        }
    }

    async fn spawn_stdio(config: McpServerConfig) -> Result<Self, McpError> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);
        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        // Use pseudo-terminal for better process management
        #[cfg(unix)]
        cmd.stdin(std::process::Stdio::piped());
        #[cfg(unix)]
        cmd.stdout(std::process::Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| McpError::SpawnFailed(config.command.clone(), e.to_string()))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| McpError::TransportError("Failed to capture stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| McpError::TransportError("Failed to capture stdout".to_string()))?;

        let (req_tx, req_rx) = mpsc::channel(32);
        let (res_tx, res_rx) = mpsc::channel(32);

        // Spawn JSON-RPC reader/writer tasks
        tokio::spawn(Self::rpc_reader(stdout, res_tx));
        tokio::spawn(Self::rpc_writer(stdin, req_rx));

        let handle = Self::new_handle(
            config,
            child,
            req_tx,
            res_rx,
        );

        Ok(handle)
    }

    async fn spawn_tcp(config: McpServerConfig) -> Result<Self, McpError> {
        // TCP transport implementation
        Err(McpError::TransportError("TCP transport not yet implemented".to_string()))
    }

    fn new_handle(
        config: McpServerConfig,
        child: Child,
        stdin: mpsc::Sender<McpRequest>,
        stdout: mpsc::Receiver<McpResponse>,
    ) -> Self {
        Self {
            id: config.id,
            name: config.name,
            transport: config.transport,
            child: Mutex::new(Some(child)),
            stdin,
            stdout,
            connected: RwLock::new(true),
        }
    }

    async fn rpc_reader(
        mut stdout: tokio::process::ChildStdout,
        tx: mpsc::Sender<McpResponse>,
    ) {
        let mut buf = Vec::new();
        let mut read_buf = [0u8; 4096];

        loop {
            match stdout.read(&mut read_buf).await {
                Ok(0) => break, // EOF
                Ok(n) => {
                    buf.extend_from_slice(&read_buf[..n]);
                    // Try to parse complete JSON-RPC messages
                    while let Some(end) = buf.windows(2).position(|w| w == [b'\r', b'\n']) {
                        let line = buf[..end].trim_ascii();
                        if !line.is_empty() {
                            if let Ok(response) = serde_json::from_slice::<McpResponse>(line) {
                                let _ = tx.send(response).await;
                            }
                        }
                        buf = buf[end + 2..].to_vec();
                    }
                }
                Err(e) => {
                    tracing::error!("MCP stdout read error: {}", e);
                    break;
                }
            }
        }
    }

    async fn rpc_writer(
        mut stdin: tokio::process::ChildStdin,
        mut rx: mpsc::Receiver<McpRequest>,
    ) {
        while let Some(request) = rx.recv().await {
            let json = serde_json::to_string(&request).unwrap();
            let line = format!("{}\r\n", json);
            if stdin.write_all(line.as_bytes()).await.is_err() {
                break;
            }
        }
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    pub async fn kill(&self) -> Result<(), McpError> {
        let mut connected = self.connected.write().await;
        *connected = false;

        let mut child_lock = self.child.lock().unwrap();
        if let Some(ref mut child) = child_lock.as_mut() {
            child.kill().await
                .map_err(|e| McpError::TransportError(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn call_tool(&self, tool_name: &str, args: serde_json::Value) -> Result<McpToolResult, McpError> {
        let request = McpRequest::CallTool {
            name: tool_name.to_string(),
            arguments: args,
        };

        let (tx, mut rx) = mpsc::channel(1);

        self.stdin.send(request).await
            .map_err(|_| McpError::Disconnected)?;

        let response = timeout(Duration::from_secs(30), rx.recv())
            .await
            .map_err(|_| McpError::Timeout)?
            .ok_or_else(|| McpError::Disconnected)?;

        match response {
            McpResponse::Result(result) => {
                // Parse the result Value into McpToolResult
                serde_json::from_value(result)
                    .map_err(|e| McpError::ParseError(e.to_string()))
            },
            McpResponse::Error(error) => Err(McpError::ToolCallFailed(error.message)),
        }
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        let request = McpRequest::ListTools {
            params: serde_json::Value::Null,
        };

        let (tx, mut rx) = mpsc::channel(1);

        self.stdin.send(request).await
            .map_err(|_| McpError::Disconnected)?;

        let response = timeout(Duration::from_secs(10), rx.recv())
            .await
            .map_err(|_| McpError::Timeout)?
            .ok_or_else(|| McpError::Disconnected)?;

        match response {
            McpResponse::Result(result) => {
                serde_json::from_value(result).map_err(|e| McpError::ParseError(e.to_string()))
            }
            McpResponse::Error(error) => Err(McpError::ToolCallFailed(error.message)),
        }
    }
}

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum McpRequest {
    #[serde(rename = "initialize")]
    Initialize {
        params: serde_json::Value,
    },
    #[serde(rename = "tools/list")]
    ListTools {
        params: serde_json::Value,
    },
    #[serde(rename = "tools/call")]
    CallTool {
        name: String,
        arguments: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

impl McpRequest {
    pub fn method(&self) -> &str {
        match self {
            McpRequest::Initialize { .. } => "initialize",
            McpRequest::ListTools { .. } => "tools/list",
            McpRequest::CallTool { name, .. } => name,
            McpRequest::Other => "unknown",
        }
    }
}

/// JSON-RPC response
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum McpResponse {
    #[serde(rename = "result")]
    Result(serde_json::Value),
    #[serde(rename = "error")]
    Error(McpErrorResponse),
}

impl McpResponse {
    pub fn into_result(self) -> Result<serde_json::Value, McpErrorResponse> {
        match self {
            McpResponse::Result(v) => Ok(v),
            McpResponse::Error(e) => Err(e),
        }
    }
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

impl McpToolResult {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![McpContent {
                content_type: "text".to_string(),
                text: Some(text.into()),
            }],
            is_error: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct McpErrorResponse {
    pub code: i32,
    pub message: String,
}

/// MCP server info for listing
#[derive(Debug, Clone, Serialize)]
pub struct McpServerInfo {
    pub id: String,
    pub name: String,
    pub transport: McpTransport,
    pub status: String,
}

/// MCP errors
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Server not found: {0}")]
    NotFound(String),

    #[error("Spawn failed: {0}")]
    SpawnFailed(String, String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Disconnected")]
    Disconnected,

    #[error("Timeout")]
    Timeout,

    #[error("Tool call failed: {0}")]
    ToolCallFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

// Sealed trait for McpServer
pub trait McpServer: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn transport(&self) -> McpTransport;
    fn is_connected(&self) -> bool;
}

/// Extension for MCP servers (placeholder for additional MCP-specific functionality)
pub trait McpExtension: McpServer {
    fn server_info(&self) -> McpServerInfo;
}

use std::collections::HashMap;
