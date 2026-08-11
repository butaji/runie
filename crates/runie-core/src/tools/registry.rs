//! Tool registry: lookup by name and per-tool execution mode.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{AgentTool, Model, ToolExecutionMode};

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn AgentTool>>,
    mcp_stdio: Vec<Arc<crate::tools::McpStdioActor>>,
    mcp_http: Vec<Arc<crate::tools::McpHttpActor>>,
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
            mcp_http: Vec::new(),
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

    pub async fn register_mcp_http(
        &mut self,
        client: crate::tools::McpHttpClient,
    ) -> Result<usize, String> {
        let owner = Arc::new(crate::tools::McpHttpActor::new(client));
        let (name, tools) = discover_http_tools(&owner).await?;
        let call = http_call_hook(owner.clone());
        let result = self.register_mcp_server(crate::tools::McpServer { name, tools }, call);
        if result.is_ok() {
            self.mcp_http.push(owner);
        } else {
            let _ = owner.as_ref().clone().close().await;
        }
        result
    }

    pub fn mcp_stdio_statuses(&self) -> Vec<crate::tools::McpStdioStatus> {
        self.mcp_stdio.iter().map(|owner| owner.status()).collect()
    }

    pub fn mcp_http_statuses(&self) -> Vec<crate::tools::McpHttpStatus> {
        self.mcp_http.iter().map(|owner| owner.status()).collect()
    }

    pub fn mcp_status_rows(&self) -> Vec<crate::tools::McpStatusRow> {
        self.mcp_stdio_statuses()
            .into_iter()
            .enumerate()
            .map(|(index, status)| crate::tools::McpStatusRow {
                transport: "stdio".into(),
                index,
                status: status.wire_name().into(),
            })
            .chain(
                self.mcp_http_statuses()
                    .into_iter()
                    .enumerate()
                    .map(|(index, status)| crate::tools::McpStatusRow {
                        transport: "http".into(),
                        index,
                        status: status.wire_name().into(),
                    }),
            )
            .collect()
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

async fn discover_http_tools(
    owner: &crate::tools::McpHttpActor,
) -> Result<(String, Vec<crate::tools::McpToolSpec>), String> {
    let initialize = owner
        .request(http_request(
            1,
            "initialize",
            serde_json::json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {"name": "runie", "version": "0"}
                }),
        ))
        .await?;
    let name = initialize["result"]["serverInfo"]["name"]
        .as_str()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("http")
        .to_owned();
    let listed = owner
        .request(http_request(2, "tools/list", serde_json::json!({})))
        .await?;
    let tools = serde_json::from_value(listed["result"]["tools"].clone())
        .map_err(|error| format!("MCP HTTP tools/list: {error}"))?;
    Ok((name, tools))
}

fn http_request(id: u64, method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

fn http_call_hook(owner: Arc<crate::tools::McpHttpActor>) -> crate::tools::McpCallHook {
    let next_id = Arc::new(std::sync::atomic::AtomicU64::new(3));
    Arc::new(move |request| {
        let owner = owner.clone();
        let next_id = next_id.clone();
        Box::pin(async move {
            let id = next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            owner
                .request(http_request(
                    id,
                    "tools/call",
                    serde_json::json!({
                        "name": request.tool, "arguments": request.arguments
                    }),
                ))
                .await
        })
    })
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

    #[tokio::test]
    async fn register_mcp_http_discovers_and_reuses_owned_session() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        // OWNER: registry integration test joins this server task below.
        let task = tokio::spawn(http_registry_fixture(listener));
        let client = crate::tools::McpHttpClient::new(
            format!("http://{address}"),
            None,
            std::time::Duration::from_secs(1),
        )
        .unwrap();
        let mut registry = ToolRegistry::new();
        assert_eq!(registry.register_mcp_http(client).await.unwrap(), 1);
        assert_eq!(
            registry.mcp_http_statuses(),
            vec![crate::tools::McpHttpStatus::Ready]
        );
        assert_eq!(
            registry.mcp_status_rows(),
            vec![crate::tools::McpStatusRow {
                transport: "http".into(),
                index: 0,
                status: "ready".into(),
            }]
        );
        let tool = registry.lookup("mcp__demo-http__echo").unwrap();
        assert_eq!(
            tool.execute("1", serde_json::json!({"x":1}), None, None)
                .await
                .unwrap()
                .details["result"]["value"],
            7
        );
        task.await.unwrap();
    }

    async fn http_registry_fixture(listener: tokio::net::TcpListener) {
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 4096];
            let size = tokio::io::AsyncReadExt::read(&mut socket, &mut request)
                .await
                .unwrap();
            let request = String::from_utf8_lossy(&request[..size]);
            let body = if request.contains("initialize") {
                r#"{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"demo-http"}}}"#
            } else if request.contains("tools/list") {
                r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","inputSchema":{"type":"object"}}]}}"#
            } else {
                r#"{"jsonrpc":"2.0","id":3,"result":{"value":7}}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nMcp-Session-Id: http-session\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
                body.len(), body
            );
            tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes())
                .await
                .unwrap();
        }
    }
}
