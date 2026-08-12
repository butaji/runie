use super::McpStdioClient;

pub(crate) fn stdio_identity(client: &McpStdioClient) -> String {
    std::iter::once(client.command.as_str())
        .chain(client.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
}
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// A single actor-owned MCP stdio process/session.
pub struct McpStdioSession {
    child: tokio::process::Child,
    input: tokio::process::ChildStdin,
    lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    timeout: Duration,
    initialized: bool,
    next_id: u64,
    pending_notifications: Vec<serde_json::Value>,
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
            pending_notifications: Vec::new(),
        })
    }

    pub fn take_notifications(&mut self) -> Vec<serde_json::Value> {
        std::mem::take(&mut self.pending_notifications)
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
        let expected = requests
            .iter()
            .filter_map(|request| request.get("id").and_then(serde_json::Value::as_u64))
            .collect::<Vec<_>>();
        let mut responses = Vec::new();
        while responses.len() < expected.len() {
            let line = tokio::time::timeout(self.timeout, self.lines.next_line())
                .await
                .map_err(|_| "MCP request timed out".to_owned())?
                .map_err(|error| error.to_string())?
                .ok_or("MCP process closed stdout")?;
            let value = serde_json::from_str::<serde_json::Value>(&line)
                .map_err(|error| format!("invalid MCP JSON: {error}"))?;
            if matches_response(&value, &expected, &responses) {
                responses.push(value);
            } else if value.get("id").is_none() {
                self.pending_notifications.push(value);
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
        super::response_result_or_error(&responses, initialize_id, "initialize")?;
        super::response_result_or_error(&responses, list_id, "tools/list")?;
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
        let responses = self.send(&[serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":arguments}})]).await?;
        super::response_result_or_error(&responses, id, "tools/call").cloned()
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

fn matches_response(
    value: &serde_json::Value,
    expected: &[u64],
    responses: &[serde_json::Value],
) -> bool {
    value
        .get("id")
        .and_then(serde_json::Value::as_u64)
        .is_some_and(|id| {
            expected.contains(&id)
                && !responses.iter().any(|response| {
                    response.get("id").and_then(serde_json::Value::as_u64) == Some(id)
                })
        })
}
