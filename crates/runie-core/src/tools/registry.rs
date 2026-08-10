//! Tool registry: lookup by name and per-tool execution mode.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{AgentTool, Model, ToolExecutionMode};

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn AgentTool>>,
    mcp_stdio: Vec<Arc<crate::tools::McpStdioActor>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            mcp_stdio: Vec::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn AgentTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn register_mcp_server(
        &mut self,
        server: crate::tools::McpServer,
        call: crate::tools::McpCallHook,
    ) -> Result<usize, String> {
        let crate::tools::McpServer { name, tools: specs } = server;
        let tools: Result<Vec<_>, _> = specs
            .into_iter()
            .map(|spec| crate::tools::McpTool::new(&name, spec, call.clone()))
            .collect();
        let tools = tools?;
        if tools
            .iter()
            .any(|tool| self.tools.contains_key(tool.name()))
        {
            return Err("an MCP tool with that qualified name is already registered".into());
        }
        let count = tools.len();
        for tool in tools {
            self.register(Arc::new(tool));
        }
        Ok(count)
    }

    pub async fn register_mcp_stdio(
        &mut self,
        client: crate::tools::McpStdioClient,
    ) -> Result<usize, String> {
        let server = client.discover().await?;
        let owner = Arc::new(crate::tools::McpStdioActor::new_persistent(client));
        let owner_for_call = owner.clone();
        let call: crate::tools::McpCallHook = Arc::new(move |request| {
            let owner = owner_for_call.clone();
            Box::pin(async move { owner.call_tool(request.tool, request.arguments).await })
        });
        self.mcp_stdio.push(owner);
        self.register_mcp_server(server, call)
    }

    pub fn mcp_stdio_statuses(&self) -> Vec<crate::tools::McpStdioStatus> {
        self.mcp_stdio.iter().map(|owner| owner.status()).collect()
    }

    pub fn lookup(&self, name: &str) -> Option<Arc<dyn AgentTool>> {
        self.tools.get(name).cloned()
    }

    pub fn execution_mode(&self, name: &str) -> Option<ToolExecutionMode> {
        self.tools.get(name).and_then(|t| t.execution_mode())
    }

    pub fn resource_key(&self, name: &str, args: &serde_json::Value) -> Option<String> {
        self.tools
            .get(name)
            .and_then(|tool| tool.resource_key(args))
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn tools(&self) -> Vec<Arc<dyn AgentTool>> {
        self.ordered_tools()
    }

    pub fn tools_for_model(&self, model: &Model) -> Vec<Arc<dyn AgentTool>> {
        self.ordered_tools()
            .into_iter()
            .filter(|tool| {
                tool.required_input()
                    .is_none_or(|kind| model.supports_input(kind))
            })
            .collect()
    }

    fn ordered_tools(&self) -> Vec<Arc<dyn AgentTool>> {
        self.names()
            .into_iter()
            .filter_map(|name| self.tools.get(&name).cloned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentToolResult, ToolResultContent};

    struct EchoTool {
        required: Option<crate::types::InputKind>,
    }
    #[async_trait::async_trait]
    impl AgentTool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn label(&self) -> &str {
            "Echo"
        }
        fn description(&self) -> &str {
            "Echoes input."
        }
        fn required_input(&self) -> Option<crate::types::InputKind> {
            self.required
        }
        async fn execute(
            &self,
            _id: &str,
            args: serde_json::Value,
            _signal: Option<tokio_util::sync::CancellationToken>,
            _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
        ) -> Result<AgentToolResult, String> {
            Ok(AgentToolResult {
                content: vec![ToolResultContent::Text {
                    text: args.to_string(),
                }],
                details: serde_json::Value::Null,
                usage: None,
                added_tool_names: vec![],
                terminate: false,
            })
        }
    }

    #[test]
    fn register_and_lookup() {
        let mut r = ToolRegistry::new();
        r.register(Arc::new(EchoTool { required: None }));
        assert!(r.lookup("echo").is_some());
        assert_eq!(r.len(), 1);
        assert_eq!(r.execution_mode("echo"), None);
    }

    #[test]
    fn register_mcp_server_adds_qualified_tools_atomically() {
        let mut registry = ToolRegistry::new();
        let server = crate::tools::McpServer {
            name: "files".into(),
            tools: vec![crate::tools::McpToolSpec {
                name: "list".into(),
                description: "List".into(),
                input_schema: serde_json::json!({"type":"object"}),
            }],
        };
        let hook: crate::tools::McpCallHook =
            Arc::new(|_| Box::pin(async { Ok(serde_json::json!({})) }));
        assert_eq!(registry.register_mcp_server(server, hook).unwrap(), 1);
        assert!(registry.lookup("mcp__files__list").is_some());
    }

    #[test]
    fn tool_projections_are_sorted_data() {
        let mut registry = ToolRegistry::new();
        let server = crate::tools::McpServer {
            name: "files".into(),
            tools: vec![
                crate::tools::McpToolSpec {
                    name: "write".into(),
                    description: "Write".into(),
                    input_schema: serde_json::json!({"type":"object"}),
                },
                crate::tools::McpToolSpec {
                    name: "inspect".into(),
                    description: "Inspect".into(),
                    input_schema: serde_json::json!({"type":"object"}),
                },
            ],
        };
        let hook: crate::tools::McpCallHook =
            Arc::new(|_| Box::pin(async { Ok(serde_json::json!({})) }));
        registry.register_mcp_server(server, hook).unwrap();
        let names: Vec<_> = registry
            .tools()
            .into_iter()
            .map(|tool| tool.name().to_owned())
            .collect();
        assert_eq!(names, ["mcp__files__inspect", "mcp__files__write"]);
    }

    #[test]
    fn model_tool_projection_filters_required_modalities() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool {
            required: Some(crate::types::InputKind::Image),
        }));
        let text_model = Model::default();
        let image_model = Model {
            input: vec![
                crate::types::InputKind::Text,
                crate::types::InputKind::Image,
            ],
            ..Model::default()
        };
        assert!(registry.tools_for_model(&text_model).is_empty());
        assert_eq!(registry.tools_for_model(&image_model).len(), 1);
    }

    #[tokio::test]
    async fn register_mcp_stdio_discovers_and_binds_owner_calls() {
        let script = "while IFS= read -r line; do case \"$line\" in *initialize*) echo '{\"id\":1,\"result\":{\"serverInfo\":{\"name\":\"demo\"}}}';; *tools/list*) echo '{\"id\":2,\"result\":{\"tools\":[{\"name\":\"echo\",\"inputSchema\":{\"type\":\"object\"}}]}}';; *tools/call*) echo '{\"id\":3,\"result\":{\"value\":7}}';; esac; done";
        let client = crate::tools::McpStdioClient::new(
            "sh",
            vec!["-c".into(), script.into()],
            std::time::Duration::from_secs(1),
        )
        .unwrap();
        let mut registry = ToolRegistry::new();
        assert_eq!(registry.register_mcp_stdio(client).await.unwrap(), 1);
        assert_eq!(
            registry.mcp_stdio_statuses(),
            vec![crate::tools::McpStdioStatus::Ready]
        );
        let tool = registry.lookup("mcp__demo__echo").unwrap();
        assert_eq!(
            tool.execute("1", serde_json::json!({"x":1}), None, None)
                .await
                .unwrap()
                .details["value"],
            7
        );
        assert_eq!(
            registry.mcp_stdio_statuses(),
            vec![crate::tools::McpStdioStatus::Ready]
        );
    }
}
