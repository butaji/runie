//! MCP is represented as data; transports and discovery stay with the host.

use crate::types::{AgentTool, AgentToolResult, ToolResultContent};
use std::sync::Arc;

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
}
