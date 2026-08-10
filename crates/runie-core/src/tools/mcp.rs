//! MCP is represented as data; transports and discovery stay with the host.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpToolSpec {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "empty_schema")]
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
        if requests.iter().any(|request| request.get("id").is_none()) {
            return Err("MCP requests must contain ids".into());
        }
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
        let mut lines = BufReader::new(stdout).lines();
        let mut responses = Vec::new();
        while responses.len() < requests.len() {
            let line = tokio::time::timeout(self.timeout, lines.next_line())
                .await
                .map_err(|_| "MCP request timed out".to_owned())?
                .map_err(|error| error.to_string())?
                .ok_or("MCP server closed stdout")?;
            let value: serde_json::Value = serde_json::from_str(&line)
                .map_err(|error| format!("invalid MCP response: {error}"))?;
            if value.get("id").is_some() {
                responses.push(value);
            }
        }
        Ok(responses)
    }
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
}
