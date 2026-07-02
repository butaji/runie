//! Central MCP Connection Manager
//!
//! Owns MCP server lifecycles with parallel startup and clean shutdown.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinSet;

use crate::config::McpServer;
use crate::mcp::cache::{CachedToolSchema, SchemaCache};

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
#[allow(dead_code)]
struct ServerHandle {
    /// Server name.
    name: String,
    /// Server configuration.
    config: McpServer,
    /// Current state.
    state: ServerState,
    /// Cancellation signal sender.
    shutdown_tx: broadcast::Sender<()>,
}

impl ServerHandle {
    fn new(name: String, config: McpServer) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            name,
            config,
            state: ServerState::Starting,
            shutdown_tx,
        }
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
    /// Background tasks.
    tasks: RwLock<JoinSet<Result<()>>>,
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
            tasks: RwLock::new(JoinSet::new()),
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
    pub async fn start_server(&self, name: String, config: McpServer) -> Result<()> {
        let handle = ServerHandle::new(name.clone(), config.clone());

        // Check cache first
        if let Some(cached) = self.cache.get(&name, &config).await {
            // Update state to running with cached tools
            let tools: Vec<McpTool> = cached.tools.into_iter().map(|ct| McpTool {
                server_name: name.clone(),
                tool: Tool::new(
                    ct.name,
                    ct.description,
                    Arc::new(ct.input_schema.as_object().cloned().unwrap_or_default()),
                ),
            }).collect();

            let mut servers = self.servers.write().await;
            let h = servers.entry(name.clone()).or_insert(handle);
            h.state = ServerState::Running(tools);
            return Ok(());
        }

        // Clone values for async task
        let name_for_task = name.clone();
        let config_for_task = config.clone();
        let cache = self.cache.clone();

        // Spawn async task to start the server
        let mut tasks = self.tasks.write().await;
        tasks.spawn(async move {
            // For stdio transport, spawn the process
            match &config_for_task.transport {
                crate::config::McpTransport::Stdio => {
                    if config_for_task.command.is_empty() {
                        return Err(anyhow::anyhow!("No command specified for stdio transport"));
                    }

                    // TODO: Implement actual MCP stdio protocol communication
                    // This is a placeholder that would connect via stdio
                    // The actual implementation would:
                    // 1. Spawn the process with stdin/stdout
                    // 2. Send tools/list request
                    // 3. Parse the response
                    // 4. Cache the results
                    tracing::info!("Starting MCP server via stdio: {:?}", config_for_task.command);

                    // Placeholder: create empty tools list
                    let tools: Vec<CachedToolSchema> = Vec::new();
                    cache.put(&name_for_task, &config_for_task, tools).await?;

                    Ok(())
                }
                crate::config::McpTransport::Http | crate::config::McpTransport::Sse => {
                    let url = config_for_task.url.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("URL required for HTTP/SSE transport")
                    })?;

                    tracing::info!("Starting MCP server via {}: {}", config_for_task.transport, url);

                    // Placeholder: would connect via HTTP/SSE
                    let tools: Vec<CachedToolSchema> = Vec::new();
                    cache.put(&name_for_task, &config_for_task, tools).await?;

                    Ok(())
                }
            }
        });

        // Store handle
        let mut servers = self.servers.write().await;
        servers.insert(name, handle);

        Ok(())
    }

    /// Stop a server gracefully.
    pub async fn stop_server(&self, name: &str) -> Result<()> {
        let mut servers = self.servers.write().await;
        if let Some(handle) = servers.get_mut(name) {
            let _ = handle.shutdown_tx.send(());
            handle.state = ServerState::Stopped;
        }
        Ok(())
    }

    /// Stop all servers and wait for tasks.
    pub async fn shutdown(&self) -> Result<()> {
        // Send shutdown to all servers
        let servers = self.servers.read().await;
        for handle in servers.values() {
            let _ = handle.shutdown_tx.send(());
        }
        drop(servers);

        // Wait for all tasks to complete
        let mut tasks = self.tasks.write().await;
        while tasks.join_next().await.is_some() {}

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
        servers.values().any(|h| matches!(h.state, ServerState::Running(_)))
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
    async fn start_server_creates_handle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = McpConnectionManager::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let config = McpServer {
            transport: crate::config::McpTransport::Stdio,
            command: vec!["echo".to_string(), "test".to_string()],
            url: None,
            headers: Default::default(),
            scope: crate::config::ConfigScope::Global,
        };

        manager.start_server("test".to_string(), config).await.unwrap();

        let state = manager.get_server_state("test").await;
        assert!(state.is_some());
    }

    #[tokio::test]
    async fn stop_server_updates_state() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = McpConnectionManager::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        let config = McpServer {
            transport: crate::config::McpTransport::Stdio,
            command: vec!["echo".to_string(), "test".to_string()],
            url: None,
            headers: Default::default(),
            scope: crate::config::ConfigScope::Global,
        };

        manager.start_server("test".to_string(), config).await.unwrap();
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
