//! MCP is represented as data; transports and discovery stay with the host.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub const MCP_HTTP_MAX_RESPONSE_BYTES: usize = 1_048_576;
pub const MCP_MAX_STREAM_EVENTS: usize = 4_096;
pub(crate) const MCP_SESSION_HEADER: &str = "mcp-session-id";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpStatusRow {
    pub transport: String,
    pub index: usize,
    pub status: String,
}

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

/// A single actor-owned MCP stdio process/session. Unlike `McpStdioClient`,
/// this keeps the child and protocol state across calls.
pub struct McpStdioSession {
    child: tokio::process::Child,
    input: tokio::process::ChildStdin,
    lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    timeout: Duration,
    initialized: bool,
    next_id: u64,
}

impl McpStdioSession {
    pub async fn connect(client: &McpStdioClient) -> Result<Self, String> {
        let mut child = tokio::process::Command::new(&client.command)
            .args(&client.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| format!("MCP spawn: {error}"))?;
        let input = child.stdin.take().ok_or("MCP stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("MCP stdout unavailable")?;
        Ok(Self {
            child,
            input,
            lines: BufReader::new(stdout).lines(),
            timeout: client.timeout,
            initialized: false,
            next_id: 1,
        })
    }

    async fn send(
        &mut self,
        requests: &[serde_json::Value],
    ) -> Result<Vec<serde_json::Value>, String> {
        for request in requests {
            self.input
                .write_all(request.to_string().as_bytes())
                .await
                .map_err(|error| error.to_string())?;
            self.input
                .write_all(b"\n")
                .await
                .map_err(|error| error.to_string())?;
        }
        self.input
            .flush()
            .await
            .map_err(|error| error.to_string())?;
        let expected_ids = requests
            .iter()
            .filter_map(|request| request.get("id").and_then(serde_json::Value::as_u64))
            .collect::<Vec<_>>();
        let mut responses = Vec::new();
        while responses.len() < expected_ids.len() {
            let line = tokio::time::timeout(self.timeout, self.lines.next_line())
                .await
                .map_err(|_| "MCP request timed out".to_owned())?
                .map_err(|error| error.to_string())?;
            let Some(line) = line else {
                return Err("MCP process closed stdout".into());
            };
            let value = serde_json::from_str::<serde_json::Value>(&line)
                .map_err(|error| format!("invalid MCP JSON: {error}"))?;
            if value
                .get("id")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|id| {
                    expected_ids.contains(&id)
                        && !responses.iter().any(|response: &serde_json::Value| {
                            response.get("id").and_then(serde_json::Value::as_u64) == Some(id)
                        })
                })
            {
                responses.push(value);
            }
        }
        Ok(responses)
    }

    async fn initialize(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }
        let initialize_id = self.next_id;
        self.next_id += 1;
        let list_id = self.next_id;
        self.next_id += 1;
        let responses = self.send(&[
            serde_json::json!({"jsonrpc":"2.0","id":initialize_id,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"runie","version":"0.1.0"}}}),
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            serde_json::json!({"jsonrpc":"2.0","id":list_id,"method":"tools/list","params":{}}),
        ]).await?;
        response_result_or_error(&responses, initialize_id, "initialize")?;
        response_result_or_error(&responses, list_id, "tools/list")?;
        self.initialized = true;
        Ok(())
    }

    pub async fn call_tool(
        &mut self,
        tool: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if tool.trim().is_empty() {
            return Err("MCP tool name must not be empty".into());
        }
        self.initialize().await?;
        let id = self.next_id;
        self.next_id += 1;
        let responses = self
            .send(&[serde_json::json!({
                "jsonrpc":"2.0", "id":id, "method":"tools/call",
                "params":{"name":tool,"arguments":arguments}
            })])
            .await?;
        response_result_or_error(&responses, id, "tools/call").cloned()
    }

    pub async fn close(mut self) -> Result<(), String> {
        self.child.kill().await.map_err(|error| error.to_string())?;
        self.child
            .wait()
            .await
            .map_err(|error| error.to_string())
            .map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpStdioStatus {
    Ready,
    Busy,
    Failed,
    Closed,
}

enum McpStdioCommand {
    Call {
        tool: String,
        arguments: serde_json::Value,
        reply: tokio::sync::oneshot::Sender<Result<serde_json::Value, String>>,
    },
    Close {
        reply: tokio::sync::oneshot::Sender<()>,
    },
}

#[derive(Clone)]
pub struct McpStdioActor {
    tx: tokio::sync::mpsc::Sender<McpStdioCommand>,
    status: tokio::sync::watch::Receiver<McpStdioStatus>,
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
        let (status_tx, status) = tokio::sync::watch::channel(McpStdioStatus::Ready);
        let (tx, owner) = crate::spawn_actor_worker!(32, move |rx: tokio::sync::mpsc::Receiver<
            McpStdioCommand,
        >| async move {
            run_stdio_worker(rx, client, persistent, status_tx).await
        });
        Self {
            tx,
            status,
            _owner: owner,
        }
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
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpHttpClient {
    pub(crate) endpoint: String,
    pub(crate) bearer_token: Option<String>,
    pub(crate) timeout: Duration,
}

impl McpHttpClient {
    pub fn new(
        endpoint: impl Into<String>,
        bearer_token: Option<String>,
        timeout: Duration,
    ) -> Result<Self, String> {
        let endpoint = endpoint.into();
        if endpoint.trim().is_empty() {
            return Err("MCP endpoint must not be empty".into());
        }
        if timeout.is_zero() {
            return Err("MCP timeout must be positive".into());
        }
        Ok(Self {
            endpoint,
            bearer_token,
            timeout,
        })
    }

    pub async fn request(&self, request: serde_json::Value) -> Result<serde_json::Value, String> {
        self.request_with_session(request, None)
            .await
            .map(|(value, _)| value)
    }

    pub(crate) async fn request_with_session(
        &self,
        request: serde_json::Value,
        session_id: Option<&str>,
    ) -> Result<(serde_json::Value, Option<String>), String> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|error| error.to_string())?;
        let mut call = client.post(&self.endpoint).json(&request);
        if let Some(token) = &self.bearer_token {
            call = call.bearer_auth(token);
        }
        if let Some(session_id) = session_id {
            call = call.header(MCP_SESSION_HEADER, session_id);
        }
        let response = call
            .send()
            .await
            .map_err(|error| format!("MCP HTTP request: {error}"))?;
        decode_http_response(response).await
    }
}

async fn decode_http_response(
    response: reqwest::Response,
) -> Result<(serde_json::Value, Option<String>), String> {
    let status = response.status();
    let response_session = response
        .headers()
        .get(MCP_SESSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = response
        .bytes()
        .await
        .map_err(|error| format!("MCP HTTP body: {error}"))?;
    if body.len() > MCP_HTTP_MAX_RESPONSE_BYTES {
        return Err(format!(
            "MCP HTTP response exceeds {} bytes",
            MCP_HTTP_MAX_RESPONSE_BYTES
        ));
    }
    if !status.is_success() {
        return Err(format!(
            "MCP HTTP status {status}: {}",
            String::from_utf8_lossy(&body)
        ));
    }
    serde_json::from_slice(&body)
        .map(|value| (value, response_session))
        .map_err(|error| format!("invalid MCP HTTP response: {error}"))
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
        return read_responses(stdout, expected, self.timeout).await;
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
mod tests {
    use super::*;
    #[test]
    fn server_and_tool_are_data_with_stable_names() {
        let server = McpServer {
            name: "files".into(),
            tools: vec![McpToolSpec {
                name: "list".into(),
                description: "List files".into(),
                input_schema: empty_schema(),
            }],
        };
        assert_eq!(server.tools[0].name, "list");
    }
    #[tokio::test]
    async fn mcp_tool_forwards_a_typed_call_to_its_owner() {
        let tool = McpTool::new(
            "files",
            McpToolSpec {
                name: "list".into(),
                description: "List files".into(),
                input_schema: empty_schema(),
            },
            Arc::new(|call| {
                Box::pin(async move {
                    Ok(serde_json::json!({"tool": call.tool, "args": call.arguments}))
                })
            }),
        )
        .unwrap();
        assert_eq!(tool.qualified_name(), "mcp__files__list");
        assert_eq!(tool.name(), "mcp__files__list");
        let result = tool
            .execute("1", serde_json::json!({"path":"."}), None, None)
            .await
            .unwrap();
        assert_eq!(result.details["tool"], "list");
        assert_eq!(result.details["args"]["path"], ".");
    }

    #[tokio::test]
    async fn stdio_actor_has_an_explicit_awaited_close_boundary() {
        let actor = McpStdioActor::new(
            McpStdioClient::new(
                "sh",
                vec!["-c".into(), "exit 0".into()],
                Duration::from_secs(1),
            )
            .unwrap(),
        );
        assert_eq!(actor.status(), McpStdioStatus::Ready);
        actor.clone().close().await.unwrap();
        assert_eq!(actor.status(), McpStdioStatus::Closed);
    }

    #[tokio::test]
    async fn stdio_transport_round_trips_json_and_ignores_notifications() {
        let script = "while IFS= read -r line; do case \"$line\" in *\\\"id\\\":1*) echo '{\"method\":\"notice\"}'; echo '{\"id\":1,\"result\":{\"ok\":true}}';; esac; done";
        let client = McpStdioClient::new(
            "sh",
            vec!["-c".into(), script.into()],
            Duration::from_secs(1),
        )
        .unwrap();
        let responses = client
            .request(&[serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize"})])
            .await
            .unwrap();
        assert_eq!(responses[0]["result"]["ok"], true);
    }

    #[tokio::test]
    async fn stdio_discovery_reduces_initialize_and_tool_list_to_server_data() {
        let script = "while IFS= read -r line; do case \"$line\" in *\\\"id\\\":1*) echo '{\"id\":1,\"result\":{\"serverInfo\":{\"name\":\"demo\"}}}';; *\\\"id\\\":2*) echo '{\"id\":2,\"result\":{\"tools\":[{\"name\":\"echo\",\"description\":\"Echo\",\"inputSchema\":{\"type\":\"object\"}}]}}';; esac; done";
        let client = McpStdioClient::new(
            "sh",
            vec!["-c".into(), script.into()],
            Duration::from_secs(1),
        )
        .unwrap();
        let server = client.discover().await.unwrap();
        assert_eq!(server.name, "demo");
        assert_eq!(server.tools[0].name, "echo");
        assert_eq!(server.tools[0].input_schema["type"], "object");
    }

    #[tokio::test]
    async fn stdio_call_reduces_tools_call_result() {
        let script = "while IFS= read -r line; do case \"$line\" in *tools/call*) echo '{\"id\":1,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"ok\"}]}}';; esac; done";
        let client = McpStdioClient::new(
            "sh",
            vec!["-c".into(), script.into()],
            Duration::from_secs(1),
        )
        .unwrap();
        let result = client
            .call_tool("echo", serde_json::json!({"value":7}))
            .await
            .unwrap();
        assert_eq!(result["content"][0]["text"], "ok");
    }

    #[tokio::test]
    async fn stdio_call_preserves_json_rpc_error_details() {
        let script = "while IFS= read -r line; do case \"$line\" in *tools/call*) echo '{\"id\":1,\"error\":{\"code\":-32602,\"message\":\"bad arguments\"}}';; esac; done";
        let client = McpStdioClient::new(
            "sh",
            vec!["-c".into(), script.into()],
            Duration::from_secs(1),
        )
        .unwrap();
        let error = client
            .call_tool("echo", serde_json::json!({}))
            .await
            .unwrap_err();
        assert_eq!(error, "MCP error -32602: bad arguments");
    }

    #[tokio::test]
    async fn stdio_actor_projects_failed_call_status() {
        let script = "while IFS= read -r line; do case \"$line\" in *tools/call*) echo '{\"id\":1,\"error\":{\"code\":-32602,\"message\":\"bad arguments\"}}';; esac; done";
        let actor = McpStdioActor::new(
            McpStdioClient::new(
                "sh",
                vec!["-c".into(), script.into()],
                Duration::from_secs(1),
            )
            .unwrap(),
        );
        assert!(actor
            .call_tool("echo", serde_json::json!({}))
            .await
            .is_err());
        assert_eq!(actor.status(), McpStdioStatus::Failed);
        actor.close().await.unwrap();
    }

    #[tokio::test]
    async fn persistent_session_initializes_once_and_reuses_process() {
        let script = r#"count=0; calls=0; while IFS= read -r line; do case "$line" in *notifications/initialized*) :;; *initialize*) count=$((count+1)); echo '{"id":1,"result":{}}';; *tools/list*) echo '{"id":2,"result":{"tools":[]}}';; *tools/call*) calls=$((calls+1)); id=$((calls+2)); echo "{\"id\":$id,\"result\":{\"count\":$count}}";; esac; done"#;
        let client = McpStdioClient::new(
            "sh",
            vec!["-c".into(), script.into()],
            Duration::from_secs(1),
        )
        .unwrap();
        let mut session = McpStdioSession::connect(&client).await.unwrap();
        assert_eq!(
            session
                .call_tool("echo", serde_json::json!({}))
                .await
                .unwrap()["count"],
            1
        );
        assert_eq!(
            session
                .call_tool("echo", serde_json::json!({}))
                .await
                .unwrap()["count"],
            1
        );
        session.close().await.unwrap();
    }
}
