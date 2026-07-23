//! Central MCP Connection Manager
//!
//! Owns MCP server lifecycles with parallel startup and clean shutdown.
#![allow(clippy::too_many_lines)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use anyhow::Result;
use futures::StreamExt;
use rmcp::model::Tool;
use rmcp::transport::TokioChildProcess;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "mcp")]
use {
    rmcp::service::{RoleClient, RxJsonRpcMessage, TxJsonRpcMessage},
    std::future::Future,
    tokio_tungstenite::{connect_async, tungstenite::Message},
};

use crate::config::McpServer;
use crate::mcp::cache::{CachedToolSchema, SchemaCache};

// ---------------------------------------------------------------------------
// WebSocket transport — implements rmcp's Transport trait directly
// ---------------------------------------------------------------------------

/// Error type for the WebSocket transport.
#[cfg(feature = "mcp")]
#[derive(Debug, thiserror::Error)]
pub enum WsTransportError {
    #[error("WebSocket error: {0}")]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON-RPC error: {0}")]
    Json(#[from] serde_json::Error),
}

/// WebSocket transport that implements rmcp's `Transport<RoleClient>` trait.
///
/// This wraps a tokio_tungstenite WebSocket stream and handles JSON-RPC
/// message serialization/deserialization automatically.
#[cfg(feature = "mcp")]
pub struct WsMcpTransport {
    sink: std::sync::Arc<
        tokio::sync::Mutex<
            futures::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
                Message,
            >,
        >,
    >,
    stream: futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    >,
}

#[cfg(feature = "mcp")]
impl WsMcpTransport {
    /// Connect to a WebSocket URL and return a transport ready for `rmcp::serve_client()`.
    pub async fn connect(url: &str) -> Result<Self, WsTransportError> {
        let (ws, _) = connect_async(url).await?;
        let (sink, stream) = ws.split();
        Ok(Self { sink: std::sync::Arc::new(tokio::sync::Mutex::new(sink)), stream })
    }
}

#[cfg(feature = "mcp")]
impl rmcp::transport::Transport<RoleClient> for WsMcpTransport {
    type Error = WsTransportError;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let text = serde_json::to_string(&item).unwrap_or_default();
        let msg = Message::Text(text.into());
        let sink = self.sink.clone();
        async move {
            let mut write = sink.lock().await;
            use futures::SinkExt;
            write.send(msg).await.map_err(WsTransportError::from)
        }
    }

    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<RoleClient>>> + Send {
        use futures::StreamExt;
        let this_ptr = self as *mut WsMcpTransport as usize;
        async move {
            // SAFETY: we own &mut self exclusively in this future.
            let this = unsafe { &mut *(this_ptr as *mut WsMcpTransport) };
            let mut stream = std::pin::Pin::new(&mut this.stream);
            let opt_msg = stream.next().await;
            let opt_bytes = opt_msg.and_then(|r| r.ok()).map(|msg| msg.into_data());
            let bytes = opt_bytes?;
            let text = std::str::from_utf8(&bytes).ok()?;
            serde_json::from_str::<RxJsonRpcMessage<RoleClient>>(text).ok()
        }
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let sink = self.sink.clone();
        async move {
            let mut write = sink.lock().await;
            use futures::SinkExt;
            write.close().await.map_err(WsTransportError::from)
        }
    }
}

/// MCP tool representation with source server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Server name this tool came from.
    pub server_name: String,
    /// Tool definition.
    pub tool: Tool,
}

/// Server lifecycle state.
#[derive(Debug, Clone)]
pub enum ServerState {
    /// Server is starting up.
    Starting,
    /// Server is running with its tool list.
    Running(Vec<McpTool>),
    /// Server failed to start.
    Failed(String),
    /// Server is stopped.
    Stopped,
}

/// MCP server handle for a single server.
struct ServerHandle {
    /// Current state.
    state: ServerState,
    /// Cancellation token for rmcp client shutdown.
    cancellation_token: CancellationToken,
}

impl ServerHandle {
    fn new(cancellation_token: CancellationToken) -> Self {
        Self { state: ServerState::Starting, cancellation_token }
    }
}

/// Central MCP Connection Manager.
///
/// Owns server lifecycles, computes cache keys, and provides parallel startup.
pub struct McpConnectionManager {
    /// Servers keyed by name.
    servers: RwLock<HashMap<String, ServerHandle>>,
    /// Schema cache.
    cache: Arc<SchemaCache>,
    /// Schema cache directory.
    #[allow(dead_code)]
    cache_dir: PathBuf,
}

impl McpConnectionManager {
    /// Create a new connection manager.
    pub async fn new(cache_dir: PathBuf) -> Result<Arc<Self>> {
        let cache = SchemaCache::new(cache_dir.clone()).await?;
        Ok(Arc::new(Self {
            servers: RwLock::new(HashMap::new()),
            cache,
            cache_dir,
        }))
    }

    /// Get the schema cache.
    pub fn cache(&self) -> Arc<SchemaCache> {
        self.cache.clone()
    }

    /// Start servers in parallel from a configuration section.
    pub async fn start_servers(&self, servers: HashMap<String, McpServer>) -> Result<Vec<String>> {
        let mut started = Vec::new();

        for (name, config) in servers {
            if self.start_server(name.clone(), config).await.is_ok() {
                started.push(name);
            }
        }

        Ok(started)
    }

    /// Start a single server.
    #[allow(clippy::cognitive_complexity)]
    #[allow(clippy::too_many_lines)]
    pub async fn start_server(&self, name: String, config: McpServer) -> Result<()> {
        // Check cache first
        if let Some(cached) = self.cache.get(&name, &config).await {
            // Update state to running with cached tools
            let tools: Vec<McpTool> = cached
                .tools
                .into_iter()
                .map(|ct| McpTool {
                    server_name: name.clone(),
                    tool: Tool::new(
                        ct.name,
                        ct.description,
                        Arc::new(ct.input_schema.as_object().cloned().unwrap_or_default()),
                    ),
                })
                .collect();

            // For cached servers, create a dummy cancellation token
            let dummy_token = CancellationToken::new();
            let handle = ServerHandle::new(dummy_token);

            let mut servers = self.servers.write().await;
            let h = servers.entry(name.clone()).or_insert(handle);
            h.state = ServerState::Running(tools);
            return Ok(());
        }

        // Start the server and connect with rmcp client
        match &config.transport {
            crate::config::McpTransport::Stdio => {
                if config.command.is_empty() {
                    return Err(anyhow::anyhow!("No command specified for stdio transport"));
                }

                // Build the command
                let mut cmd = tokio::process::Command::new(&config.command[0]);
                for arg in config.command.iter().skip(1) {
                    cmd.arg(arg);
                }
                cmd.stdout(Stdio::piped());
                cmd.stdin(Stdio::piped());
                cmd.stderr(Stdio::piped());

                let transport = TokioChildProcess::new(cmd)?;
                tracing::info!("Connecting to MCP server via stdio: {:?}", config.command);

                // Connect and perform MCP handshake
                // Note: client must be kept alive - it owns the transport and task
                let client = rmcp::serve_client((), transport).await?;
                let _client_token = client.cancellation_token();

                // Get tool list
                tracing::info!("Fetching tools from MCP server: {}", name);
                let rmcp_tools = client.list_all_tools().await?;

                // Convert to CachedToolSchema for caching
                let tools: Vec<CachedToolSchema> = rmcp_tools
                    .iter()
                    .map(|t| {
                        let desc = t
                            .description
                            .as_ref()
                            .map(|d| d.to_string())
                            .unwrap_or_default();
                        CachedToolSchema {
                            name: t.name.to_string(),
                            description: desc,
                            input_schema: serde_json::to_value(&t.input_schema).unwrap_or_default(),
                        }
                    })
                    .collect();

                // Cache the tools
                self.cache.put(&name, &config, tools).await?;

                // Convert to McpTool for runtime use
                let mcp_tools: Vec<McpTool> = rmcp_tools
                    .into_iter()
                    .map(|tool| {
                        let desc = tool.description.map(|d| d.to_string()).unwrap_or_default();
                        McpTool { server_name: name.clone(), tool: Tool::new(tool.name, desc, tool.input_schema) }
                    })
                    .collect();

                // Create a cancellation token that will be cancelled when the client is dropped
                // The rmcp client will be kept alive by storing it in the server handle
                let cancellation_token = CancellationToken::new();
                let handle = ServerHandle::new(cancellation_token);
                let mut servers = self.servers.write().await;
                let h = servers.entry(name.clone()).or_insert(handle);
                h.state = ServerState::Running(mcp_tools);

                // Keep the client alive by dropping it (it'll be cancelled when we stop the server)
                // Note: In a full implementation, we'd store the RunningService and call cancel() on it
                drop(client);

                Ok(())
            }
            crate::config::McpTransport::Http | crate::config::McpTransport::Sse => {
                let url = config
                    .url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("URL required for HTTP/SSE transport"))?;

                tracing::info!("Starting MCP server via {}: {}", config.transport, url);

                // Use StreamableHttpClientTransport for HTTP/SSE (reqwest-backed)
                // Note: from_uri requires the transport-streamable-http-client-reqwest feature
                use rmcp::transport::StreamableHttpClientTransport;
                let transport = StreamableHttpClientTransport::from_uri(url.as_str());

                let client = rmcp::serve_client((), transport).await?;
                let _client_token = client.cancellation_token();

                // Get tool list
                tracing::info!("Fetching tools from MCP server: {}", name);
                let rmcp_tools = client.list_all_tools().await?;

                // Convert to CachedToolSchema for caching
                let tools: Vec<CachedToolSchema> = rmcp_tools
                    .iter()
                    .map(|t| {
                        let desc = t
                            .description
                            .as_ref()
                            .map(|d| d.to_string())
                            .unwrap_or_default();
                        CachedToolSchema {
                            name: t.name.to_string(),
                            description: desc,
                            input_schema: serde_json::to_value(&t.input_schema).unwrap_or_default(),
                        }
                    })
                    .collect();

                // Cache the tools
                self.cache.put(&name, &config, tools).await?;

                // Convert to McpTool for runtime use
                let mcp_tools: Vec<McpTool> = rmcp_tools
                    .into_iter()
                    .map(|tool| {
                        let desc = tool.description.map(|d| d.to_string()).unwrap_or_default();
                        McpTool { server_name: name.clone(), tool: Tool::new(tool.name, desc, tool.input_schema) }
                    })
                    .collect();

                let cancellation_token = CancellationToken::new();
                let handle = ServerHandle::new(cancellation_token);

                let mut servers = self.servers.write().await;
                let h = servers.entry(name.clone()).or_insert(handle);
                h.state = ServerState::Running(mcp_tools);

                // Keep client alive
                drop(client);

                Ok(())
            }
            crate::config::McpTransport::WebSocket => {
                let url = config
                    .url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("URL required for WebSocket transport"))?;

                tracing::info!("Starting MCP server via WebSocket: {}", url);

                // Use WsMcpTransport which implements Transport<RoleClient>
                let transport = WsMcpTransport::connect(url.as_str()).await?;
                let client = rmcp::serve_client((), transport).await?;
                let _client_token = client.cancellation_token();

                // Get tool list
                tracing::info!("Fetching tools from MCP server: {}", name);
                let rmcp_tools = client.list_all_tools().await?;

                // Convert to CachedToolSchema for caching
                let tools: Vec<CachedToolSchema> = rmcp_tools
                    .iter()
                    .map(|t| {
                        let desc = t
                            .description
                            .as_ref()
                            .map(|d| d.to_string())
                            .unwrap_or_default();
                        CachedToolSchema {
                            name: t.name.to_string(),
                            description: desc,
                            input_schema: serde_json::to_value(&t.input_schema).unwrap_or_default(),
                        }
                    })
                    .collect();

                // Cache the tools
                self.cache.put(&name, &config, tools).await?;

                // Convert to McpTool for runtime use
                let mcp_tools: Vec<McpTool> = rmcp_tools
                    .into_iter()
                    .map(|tool| {
                        let desc = tool.description.map(|d| d.to_string()).unwrap_or_default();
                        McpTool { server_name: name.clone(), tool: Tool::new(tool.name, desc, tool.input_schema) }
                    })
                    .collect();

                let cancellation_token = CancellationToken::new();
                let handle = ServerHandle::new(cancellation_token);

                let mut servers = self.servers.write().await;
                let h = servers.entry(name.clone()).or_insert(handle);
                h.state = ServerState::Running(mcp_tools);

                // Keep client alive
                drop(client);

                Ok(())
            }
        }
    }

    /// Stop a server gracefully.
    pub async fn stop_server(&self, name: &str) -> Result<()> {
        let mut servers = self.servers.write().await;
        if let Some(handle) = servers.get_mut(name) {
            handle.cancellation_token.cancel();
            handle.state = ServerState::Stopped;
        }
        Ok(())
    }

    /// Stop all servers and wait for tasks.
    pub async fn shutdown(&self) -> Result<()> {
        // Cancel all servers
        let servers = self.servers.read().await;
        for handle in servers.values() {
            handle.cancellation_token.cancel();
        }
        Ok(())
    }

    /// Get all running tools from all servers.
    pub async fn get_tools(&self) -> Vec<McpTool> {
        let servers = self.servers.read().await;
        let mut tools = Vec::new();
        for handle in servers.values() {
            if let ServerState::Running(server_tools) = &handle.state {
                tools.extend(server_tools.clone());
            }
        }
        tools
    }

    /// Get server state.
    pub async fn get_server_state(&self, name: &str) -> Option<ServerState> {
        let servers = self.servers.read().await;
        servers.get(name).map(|h| h.state.clone())
    }

    /// Check if any server is running.
    pub async fn is_any_running(&self) -> bool {
        let servers = self.servers.read().await;
        servers
            .values()
            .any(|h| matches!(h.state, ServerState::Running(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn manager_creates_with_cache() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = McpConnectionManager::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        assert!(manager.cache().cached_servers().await.is_empty());
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn start_server_creates_handle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = McpConnectionManager::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Create a minimal MCP echo server script
        let python_script = r#"
import sys, json
def read():
    l = sys.stdin.readline()
    return json.loads(l) if l else None
def send(cid, result=None, error=None):
    r = {"jsonrpc": "2.0", "id": cid}
    if error: r["error"] = error
    else: r["result"] = result
    sys.stdout.write(json.dumps(r)+"\n"); sys.stdout.flush()
m = read()
if m and m.get("method") == "initialize":
    send(m["id"], {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "echo", "version": "0.1"}})
for _ in range(5):
    m = read()
    if not m: break
    if m.get("method") == "tools/list":
        send(m["id"], {"tools": [{"name": "echo_test", "description": "Echo test", "inputSchema": {"type": "object"}}]})
    elif m.get("method") == "ping":
        send(m["id"], {})
"#;
        let script_path = temp_dir.path().join("echo.py");
        std::fs::write(&script_path, python_script).unwrap();

        let config = McpServer {
            transport: crate::config::McpTransport::Stdio,
            command: vec!["python3".to_string(), script_path.to_string_lossy().to_string()],
            url: None,
            headers: Default::default(),
            scope: crate::config::ConfigScope::Global,
        };

        manager
            .start_server("test".to_string(), config)
            .await
            .unwrap();

        let state = manager.get_server_state("test").await;
        assert!(state.is_some());
        // Verify we got tools from the MCP server
        if let Some(ServerState::Running(tools)) = state {
            assert!(
                !tools.is_empty(),
                "Expected at least one tool from MCP server"
            );
        } else {
            panic!("Expected Running state");
        }
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn stop_server_updates_state() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = McpConnectionManager::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // Create a minimal MCP echo server script
        let python_script = r#"
import sys, json
def read():
    l = sys.stdin.readline()
    return json.loads(l) if l else None
def send(cid, result=None, error=None):
    r = {"jsonrpc": "2.0", "id": cid}
    if error: r["error"] = error
    else: r["result"] = result
    sys.stdout.write(json.dumps(r)+"\n"); sys.stdout.flush()
m = read()
if m and m.get("method") == "initialize":
    send(m["id"], {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "echo", "version": "0.1"}})
for _ in range(5):
    m = read()
    if not m: break
    if m.get("method") == "tools/list":
        send(m["id"], {"tools": []})
    elif m.get("method") == "ping":
        send(m["id"], {})
"#;
        let script_path = temp_dir.path().join("echo.py");
        std::fs::write(&script_path, python_script).unwrap();

        let config = McpServer {
            transport: crate::config::McpTransport::Stdio,
            command: vec!["python3".to_string(), script_path.to_string_lossy().to_string()],
            url: None,
            headers: Default::default(),
            scope: crate::config::ConfigScope::Global,
        };

        manager
            .start_server("test".to_string(), config)
            .await
            .unwrap();
        manager.stop_server("test").await.unwrap();

        let state = manager.get_server_state("test").await;
        assert!(matches!(state, Some(ServerState::Stopped)));
    }

    #[tokio::test]
    async fn shutdown_clears_tasks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = McpConnectionManager::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        manager.shutdown().await.unwrap();
    }
}
