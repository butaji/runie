//! MCP is represented as data; transports and discovery stay with the host.
use crate::types::{AgentTool, AgentToolResult, ToolResultContent};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
pub const MCP_HTTP_MAX_RESPONSE_BYTES: usize = 1_048_576;
pub const MCP_MAX_STREAM_EVENTS: usize = 4_096;
pub(crate) const MCP_SESSION_HEADER: &str = "mcp-session-id";
macro_rules! mcp_status_wire_names {
    ($status:ty => { $($variant:ident => $wire:literal),+ $(,)? }) => {
        impl $status {
            pub const fn wire_name(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
                }
            pub fn from_wire_name(name: &str) -> Option<Self> {
                match name { $($wire => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}
#[path = "mcp_status.rs"]
mod status;
pub use status::McpStatusRow;
#[path = "mcp_transport.rs"]
mod transport;
pub use transport::McpTransport;
#[path = "mcp_http_session.rs"]
mod http_session;
pub use http_session::{McpHttpActor, McpHttpSession, McpHttpStatus};
#[path = "mcp_stream.rs"]
mod stream;
pub use stream::{
    parse_mcp_event_stream, reduce_mcp_notification_queue, reduce_mcp_stream_event,
    McpBackpressureStatus, McpConnectionStatus, McpNotificationQueue, McpNotificationQueueEvent,
    McpReconnectDecision, McpReconnectPolicy, McpReconnectState, McpStreamEvent, McpStreamSnapshot,
};
#[path = "mcp_http_stream.rs"]
mod http_stream;
#[path = "mcp_http_transport.rs"]
mod http_transport;
pub use http_transport::McpHttpClient;
#[path = "mcp_stdio_transport.rs"]
mod stdio_transport;
use stdio_transport::stdio_identity;
pub use stdio_transport::McpStdioSession;
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpToolSpec {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "empty_schema")]
    #[serde(rename = "inputSchema", alias = "input_schema")]
    pub input_schema: serde_json::Value,
}
fn empty_schema() -> serde_json::Value {
    serde_json::json!({"type": "object"})
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpServer {
    pub name: String,
    #[serde(default)]
    pub tools: Vec<McpToolSpec>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpStdioClient {
    pub command: String,
    pub args: Vec<String>,
    pub timeout: Duration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpStdioStatus {
    Ready,
    Busy,
    Failed,
    Closed,
}
mcp_status_wire_names!(McpStdioStatus => {
    Ready => "ready",
    Busy => "busy",
    Failed => "failed",
    Closed => "closed",
});
enum McpStdioCommand {
    Call {
        tool: String,
        arguments: serde_json::Value,
        reply: tokio::sync::oneshot::Sender<Result<serde_json::Value, String>>,
    },
    Close {
        reply: tokio::sync::oneshot::Sender<()>,
    },
    Reconnect {
        reply: tokio::sync::oneshot::Sender<()>,
    },
}
#[derive(Clone)]
pub struct McpStdioActor {
    tx: tokio::sync::mpsc::Sender<McpStdioCommand>,
    status: tokio::sync::watch::Receiver<McpStdioStatus>,
    identity: String,
    _owner: std::sync::Arc<crate::task_owner::TaskOwner>,
}
impl McpStdioActor {
    pub fn new(client: McpStdioClient) -> Self {
        Self::new_with_persistence(client, false)
    }
    pub fn new_persistent(client: McpStdioClient) -> Self {
        Self::new_with_persistence(client, true)
    }

    fn new_with_persistence(client: McpStdioClient, persistent: bool) -> Self {
        let identity = stdio_identity(&client);
        let (status_tx, status) = tokio::sync::watch::channel(McpStdioStatus::Ready);
        let (tx, owner) = crate::spawn_actor_worker!(32, move |rx: tokio::sync::mpsc::Receiver<
            McpStdioCommand,
        >| async move {
            run_stdio_worker(rx, client, persistent, status_tx).await
        });
        Self {
            tx,
            status,
            identity,
            _owner: owner,
        }
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn status(&self) -> McpStdioStatus {
        *self.status.borrow()
    }

    pub fn subscribe_status(&self) -> tokio::sync::watch::Receiver<McpStdioStatus> {
        self.status.clone()
    }

    pub async fn call_tool(
        &self,
        tool: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        self.tx
            .send(McpStdioCommand::Call {
                tool: tool.into(),
                arguments,
                reply,
            })
            .await
            .map_err(|_| "MCP stdio actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "MCP stdio actor response was dropped".to_owned())?
    }

    pub async fn close(self) -> Result<(), String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        self.tx
            .send(McpStdioCommand::Close { reply })
            .await
            .map_err(|_| "MCP stdio actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "MCP stdio actor close response was dropped".to_owned())
    }

    pub async fn reconnect(&self) -> Result<(), String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        self.tx
            .send(McpStdioCommand::Reconnect { reply })
            .await
            .map_err(|_| "MCP stdio actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "MCP stdio actor reconnect response was dropped".to_owned())
    }
}

async fn run_stdio_worker(
    mut rx: tokio::sync::mpsc::Receiver<McpStdioCommand>,
    client: McpStdioClient,
    persistent: bool,
    status_tx: tokio::sync::watch::Sender<McpStdioStatus>,
) {
    let mut session = None;
    while let Some(command) = rx.recv().await {
        match command {
            McpStdioCommand::Call {
                tool,
                arguments,
                reply,
            } => {
                let _ = status_tx.send(McpStdioStatus::Busy);
                let result = call_stdio(
                    &client,
                    &mut session,
                    persistent,
                    &tool,
                    arguments,
                    &status_tx,
                )
                .await;
                let _ = status_tx.send(if result.is_ok() {
                    McpStdioStatus::Ready
                } else {
                    McpStdioStatus::Failed
                });
                let _ = reply.send(result);
            }
            McpStdioCommand::Close { reply } => {
                close_stdio(reply, &mut session, &status_tx).await;
                return;
            }
            McpStdioCommand::Reconnect { reply } => {
                reconnect_stdio(&mut session, &status_tx, reply).await
            }
        }
    }
}

async fn reconnect_stdio(
    session: &mut Option<McpStdioSession>,
    status_tx: &tokio::sync::watch::Sender<McpStdioStatus>,
    reply: tokio::sync::oneshot::Sender<()>,
) {
    if let Some(session) = session.take() {
        let _ = session.close().await;
    }
    let _ = status_tx.send(McpStdioStatus::Ready);
    let _ = reply.send(());
}

async fn close_stdio(
    reply: tokio::sync::oneshot::Sender<()>,
    session: &mut Option<McpStdioSession>,
    status_tx: &tokio::sync::watch::Sender<McpStdioStatus>,
) {
    let _ = status_tx.send(McpStdioStatus::Closed);
    let _ = reply.send(());
    if let Some(session) = session.take() {
        let _ = session.close().await;
    }
}

async fn call_stdio(
    client: &McpStdioClient,
    session: &mut Option<McpStdioSession>,
    persistent: bool,
    tool: &str,
    arguments: serde_json::Value,
    status_tx: &tokio::sync::watch::Sender<McpStdioStatus>,
) -> Result<serde_json::Value, String> {
    if !persistent {
        return client.call_tool(tool, arguments).await;
    }
    if session.is_none() {
        match McpStdioSession::connect(client).await {
            Ok(value) => *session = Some(value),
            Err(error) => {
                let _ = status_tx.send(McpStdioStatus::Failed);
                return Err(error);
            }
        }
    }
    session
        .as_mut()
        .expect("persistent MCP session")
        .call_tool(tool, arguments)
        .await
}

impl McpStdioClient {
    pub fn new(
        command: impl Into<String>,
        args: Vec<String>,
        timeout: Duration,
    ) -> Result<Self, String> {
        let command = command.into();
        if command.trim().is_empty() {
            return Err("MCP command must not be empty".into());
        }
        if timeout.is_zero() {
            return Err("MCP timeout must be positive".into());
        }
        Ok(Self {
            command,
            args,
            timeout,
        })
    }

    pub async fn request(
        &self,
        requests: &[serde_json::Value],
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut child = tokio::process::Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| format!("MCP spawn: {error}"))?;
        let result = self.exchange(&mut child, requests).await;
        let _ = child.kill().await;
        let _ = child.wait().await;
        result
    }

    pub async fn discover(&self) -> Result<McpServer, String> {
        let responses = self
            .request(&[
                serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"runie","version":"0.1.0"}}}),
                serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
                serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
            ])
            .await?;
        let initialize = response_result_or_error(&responses, 1, "initialize")?;
        let server_name = initialize
            .get("serverInfo")
            .and_then(|value| value.get("name"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or(&self.command)
            .to_owned();
        let tools = response_result_or_error(&responses, 2, "tools/list")?
            .get("tools")
            .cloned()
            .ok_or("MCP tools/list response has no tools")?;
        let tools = serde_json::from_value(tools)
            .map_err(|error| format!("invalid MCP tool list: {error}"))?;
        Ok(McpServer {
            name: server_name,
            tools,
        })
    }

    pub async fn call_tool(
        &self,
        tool: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if tool.trim().is_empty() {
            return Err("MCP tool name must not be empty".into());
        }
        let responses = self
            .request(&[serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":tool,"arguments":arguments}})])
            .await?;
        response_result_or_error(&responses, 1, "tools/call").cloned()
    }

    async fn exchange(
        &self,
        child: &mut tokio::process::Child,
        requests: &[serde_json::Value],
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut input = child.stdin.take().ok_or("MCP stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("MCP stdout unavailable")?;
        for request in requests {
            input
                .write_all(request.to_string().as_bytes())
                .await
                .map_err(|error| error.to_string())?;
            input
                .write_all(b"\n")
                .await
                .map_err(|error| error.to_string())?;
        }
        input.flush().await.map_err(|error| error.to_string())?;
        drop(input);
        let expected = requests
            .iter()
            .filter(|request| request.get("id").is_some())
            .count();
        read_responses(stdout, expected, self.timeout).await
    }
}

async fn read_responses(
    stdout: tokio::process::ChildStdout,
    expected: usize,
    timeout: Duration,
) -> Result<Vec<serde_json::Value>, String> {
    let mut lines = BufReader::new(stdout).lines();
    let mut responses: Vec<serde_json::Value> = Vec::new();
    while responses.len() < expected {
        let line = tokio::time::timeout(timeout, lines.next_line())
            .await
            .map_err(|_| "MCP request timed out".to_owned())?
            .map_err(|error| error.to_string())?
            .ok_or("MCP server closed stdout")?;
        let value: serde_json::Value = serde_json::from_str(&line)
            .map_err(|error| format!("invalid MCP response: {error}"))?;
        if value.get("id").is_some()
            && !responses
                .iter()
                .any(|response| response.get("id") == value.get("id"))
        {
            responses.push(value);
        }
    }
    Ok(responses)
}

fn response_result(responses: &[serde_json::Value], id: u64) -> Option<&serde_json::Value> {
    responses
        .iter()
        .find(|response| response.get("id").and_then(serde_json::Value::as_u64) == Some(id))
        .and_then(|response| response.get("result"))
}

fn response_error(responses: &[serde_json::Value], id: u64) -> Option<String> {
    responses
        .iter()
        .find(|response| response.get("id").and_then(serde_json::Value::as_u64) == Some(id))
        .and_then(|response| response.get("error"))
        .map(|error| {
            let code = error.get("code").and_then(serde_json::Value::as_i64);
            let message = error
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown MCP error");
            code.map_or_else(
                || format!("MCP error: {message}"),
                |code| format!("MCP error {code}: {message}"),
            )
        })
}

fn response_result_or_error<'a>(
    responses: &'a [serde_json::Value],
    id: u64,
    operation: &str,
) -> Result<&'a serde_json::Value, String> {
    response_result(responses, id).ok_or_else(|| {
        response_error(responses, id)
            .unwrap_or_else(|| format!("MCP {operation} response has no result"))
    })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpCallRequest {
    pub server: String,
    pub tool: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

pub type McpCallHook = Arc<
    dyn Fn(
            McpCallRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>,
        > + Send
        + Sync,
>;

pub struct McpTool {
    server: String,
    spec: McpToolSpec,
    qualified: String,
    call: McpCallHook,
}

impl McpTool {
    pub fn new(
        server: impl Into<String>,
        spec: McpToolSpec,
        call: McpCallHook,
    ) -> Result<Self, String> {
        let server = server.into();
        if server.trim().is_empty() || spec.name.trim().is_empty() {
            return Err("MCP server and tool names must not be empty".into());
        }
        let qualified = format!("mcp__{}__{}", server, spec.name);
        Ok(Self {
            server,
            spec,
            qualified,
            call,
        })
    }
    pub fn qualified_name(&self) -> &str {
        &self.qualified
    }
}

#[async_trait::async_trait]
impl AgentTool for McpTool {
    fn name(&self) -> &str {
        &self.qualified
    }
    fn label(&self) -> &str {
        "MCP tool"
    }
    fn description(&self) -> &str {
        &self.spec.description
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(self.spec.input_schema.clone())
    }
    async fn execute(
        &self,
        _: &str,
        args: serde_json::Value,
        _: Option<tokio_util::sync::CancellationToken>,
        _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let value = (self.call)(McpCallRequest {
            server: self.server.clone(),
            tool: self.spec.name.clone(),
            arguments: args,
        })
        .await?;
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: value.to_string(),
            }],
            details: value,
            ..Default::default()
        })
    }
}
#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
