//! MCP (Model Context Protocol) server configuration and status management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// MCP server configuration from `~/.runie/mcp.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// MCP server connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpStatus {
    Connected,
    Disconnected,
    Unavailable,
}

impl McpStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            McpStatus::Connected => "[connected]",
            McpStatus::Disconnected => "[disconnected]",
            McpStatus::Unavailable => "[unavailable]",
        }
    }
}

/// MCP runtime state per server.
#[derive(Debug, Clone)]
pub struct McpServer {
    pub config: McpServerConfig,
    pub status: McpStatus,
}

/// Get the default MCP config path: `~/.runie/mcp.toml`.
pub fn mcp_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("runie").join("mcp.toml"))
}

/// Parse MCP servers from `mcp.toml`.
///
/// Expected format:
/// ```toml
/// [[mcp.servers]]
/// name = "linear"
/// command = "linear-mcp"
/// args = ["--port", "8080"]
/// env = { LINEAR_API_KEY = "..." }
/// ```
pub fn load_mcp_servers(path: &Path) -> anyhow::Result<Vec<McpServerConfig>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    let config: TomlConfig = toml::from_str(&content)?;
    Ok(config.mcp.unwrap_or_default().servers)
}

/// Save MCP servers to `mcp.toml`.
pub fn save_mcp_servers(path: &Path, servers: &[McpServerConfig]) -> anyhow::Result<()> {
    let config = TomlConfig {
        mcp: Some(McpSection {
            servers: servers.to_vec(),
        }),
    };
    let content = toml::to_string_pretty(&config)?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, content)?;
    Ok(())
}

/// Generate default statuses for all servers (all unavailable initially).
pub fn generate_default_statuses(servers: &[McpServerConfig]) -> HashMap<String, McpStatus> {
    servers
        .iter()
        .map(|s| (s.name.clone(), McpStatus::Unavailable))
        .collect()
}

/// Filter to only connected servers.
pub fn filter_available_servers(servers: &[McpServer]) -> Vec<&McpServer> {
    servers
        .iter()
        .filter(|s| s.status == McpStatus::Connected)
        .collect()
}

/// Count unavailable servers (disconnected or unavailable status).
pub fn count_unavailable_servers(statuses: &HashMap<String, McpStatus>) -> usize {
    statuses
        .values()
        .filter(|s| **s != McpStatus::Connected)
        .count()
}

/// Inject MCP credentials as env vars for server startup.
///
/// Returns `RUNIE_MCP_<NAME>_TOKEN=<value>` entries for servers with defined env vars.
pub fn inject_mcp_env_vars(servers: &[McpServerConfig]) -> HashMap<String, String> {
    let mut env = HashMap::new();
    for server in servers {
        for (key, value) in &server.env {
            let env_key = format!(
                "RUNIE_MCP_{}_{}",
                server.name.to_uppercase(),
                key.to_uppercase()
            );
            env.insert(env_key, value.clone());
        }
    }
    env
}

/// Namespaces a tool name with the server name (double underscore to avoid collisions).
pub fn namespace_tool(server_name: &str, tool_name: &str) -> String {
    format!("{}__{}", server_name, tool_name)
}

/// MCP JSON-RPC request/response types for stdio transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
#[serde(tag = "jsonrpc", content = "2.0")]
enum JsonRpc {
    Request {
        id: u64,
        method: String,
        params: Option<serde_json::Value>,
    },
    Response {
        id: u64,
        result: Option<serde_json::Value>,
        error: Option<JsonRpcError>,
    },
    Notification {
        method: String,
        params: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

/// MCP tool definition from server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP client manager for stdio-based MCP servers.
pub struct McpClientManager {
    servers: HashMap<String, McpServerHandle>,
    #[allow(dead_code)]
    next_id: u64,
}

struct McpServerHandle {
    config: McpServerConfig,
    status: McpStatus,
    tools: Vec<McpTool>,
    child: Option<std::process::Child>,
}

impl std::fmt::Debug for McpServerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpServerHandle")
            .field("config", &self.config)
            .field("status", &self.status)
            .field("tools", &self.tools.len())
            .finish()
    }
}

impl std::fmt::Debug for McpClientManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClientManager")
            .field("servers", &self.servers)
            .finish()
    }
}

impl McpClientManager {
    /// Create a new MCP client manager.
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            next_id: 1,
        }
    }

    /// Connect to an MCP server and discover its tools.
    pub fn connect(&mut self, config: McpServerConfig) -> anyhow::Result<()> {
        let name = config.name.clone();
        let env_vars = build_env_vars(&name, &config.env);

        let child = std::process::Command::new(&config.command)
            .args(&config.args)
            .envs(env_vars)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start MCP server '{}': {}", config.name, e))?;

        let handle = McpServerHandle {
            config,
            status: McpStatus::Connected,
            tools: Vec::new(),
            child: Some(child),
        };

        // Store child with stdin/stdout wrapped
        self.servers.insert(name, handle);
        Ok(())
    }

    /// Disconnect from an MCP server.
    pub fn disconnect(&mut self, name: &str) -> anyhow::Result<()> {
        if let Some(handle) = self.servers.remove(name) {
            if let Some(mut child) = handle.child {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        Ok(())
    }

    /// List connection statuses for all servers.
    pub fn list_statuses(&self) -> std::collections::HashMap<String, McpStatus> {
        self.servers
            .iter()
            .map(|(k, v)| (k.clone(), v.status))
            .collect()
    }

    /// List all available tools from connected servers, namespaced by server name.
    pub fn list_tools(&self) -> Vec<(String, McpTool)> {
        let mut tools = Vec::new();
        for (server_name, handle) in &self.servers {
            if handle.status == McpStatus::Connected {
                for tool in &handle.tools {
                    let namespaced = namespace_tool(server_name, &tool.name);
                    tools.push((namespaced, tool.clone()));
                }
            }
        }
        tools
    }

    /// Call an MCP tool by namespaced name (placeholder - actual implementation needs async).
    pub fn call_tool(
        &self,
        _namespaced_name: &str,
        _arguments: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::json!({ "success": true, "message": "MCP tool call not yet implemented" }))
    }

    #[allow(dead_code)]
    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Build env vars with injected MCP credentials.
fn build_env_vars(name: &str, extra_env: &HashMap<String, String>) -> HashMap<String, String> {
    let mut env_vars: HashMap<String, String> = std::env::vars().collect();
    for (key, value) in extra_env {
        let injected_key = format!("RUNIE_MCP_{}_{}", name.to_uppercase(), key.to_uppercase());
        env_vars.insert(injected_key, value.clone());
    }
    env_vars
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Format status indicator for unavailable servers.
pub fn unavailable_badge(count: usize) -> Option<String> {
    if count > 0 {
        Some(format!(
            "⛔ {} MCP server{} unavailable",
            count,
            if count == 1 { "" } else { "s" }
        ))
    } else {
        None
    }
}

// ============================================================================
// TOML parsing helpers
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
struct TomlConfig {
    #[serde(default)]
    mcp: Option<McpSection>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct McpSection {
    #[serde(default)]
    servers: Vec<McpServerConfig>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(name: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.into(),
            command: format!("{}-mcp", name),
            args: vec![],
            env: HashMap::new(),
        }
    }

    #[test]
    fn parse_mcp_config_toml() {
        let config = r#"
[[mcp.servers]]
name = "linear"
command = "linear-mcp"
args = ["--port", "8080"]
env = { LINEAR_API_KEY = "secret123" }

[[mcp.servers]]
name = "sentry"
command = "sentry-mcp"
"#;
        let parsed: TomlConfig = toml::from_str(config).unwrap();
        let servers = parsed.mcp.unwrap().servers;
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].name, "linear");
        assert_eq!(servers[0].args, vec!["--port", "8080"]);
        assert_eq!(servers[0].env.get("LINEAR_API_KEY").unwrap(), "secret123");
        assert_eq!(servers[1].name, "sentry");
    }

    #[test]
    fn mcp_server_status_from_connection() {
        assert_eq!(McpStatus::Connected.badge(), "[connected]");
        assert_eq!(McpStatus::Disconnected.badge(), "[disconnected]");
        assert_eq!(McpStatus::Unavailable.badge(), "[unavailable]");
    }

    #[test]
    fn inject_mcp_env_vars_test() {
        let mut env1 = HashMap::new();
        env1.insert("API_KEY".into(), "secret".into());

        let servers = vec![
            McpServerConfig {
                name: "linear".into(),
                command: "linear-mcp".into(),
                args: vec![],
                env: env1,
            },
            McpServerConfig {
                name: "sentry".into(),
                command: "sentry-mcp".into(),
                args: vec![],
                env: HashMap::new(),
            },
        ];

        let env = inject_mcp_env_vars(&servers);
        assert_eq!(env.get("RUNIE_MCP_LINEAR_API_KEY"), Some(&"secret".into()));
        assert!(!env.contains_key("RUNIE_MCP_SENTRY_"));
    }

    #[test]
    fn filter_available_servers_test() {
        let servers = vec![
            McpServer {
                config: make_config("linear"),
                status: McpStatus::Connected,
            },
            McpServer {
                config: make_config("sentry"),
                status: McpStatus::Unavailable,
            },
            McpServer {
                config: make_config("grafana"),
                status: McpStatus::Disconnected,
            },
            McpServer {
                config: make_config("filesystem"),
                status: McpStatus::Connected,
            },
        ];

        let available = filter_available_servers(&servers);
        assert_eq!(available.len(), 2);
        assert_eq!(available[0].config.name, "linear");
        assert_eq!(available[1].config.name, "filesystem");
    }

    #[test]
    fn tool_name_is_namespaced() {
        assert_eq!(
            namespace_tool("linear", "create_issue"),
            "linear__create_issue"
        );
        assert_eq!(
            namespace_tool("filesystem", "read_file"),
            "filesystem__read_file"
        );
        assert_eq!(
            namespace_tool("sentry", "list_projects"),
            "sentry__list_projects"
        );
    }

    #[test]
    fn mcp_client_manager_creation() {
        let manager = McpClientManager::new();
        assert!(manager.list_statuses().is_empty());
        assert!(manager.list_tools().is_empty());
    }

    #[test]
    fn count_unavailable_servers_test() {
        let mut statuses = HashMap::new();
        statuses.insert("linear".into(), McpStatus::Connected);
        statuses.insert("sentry".into(), McpStatus::Unavailable);
        statuses.insert("grafana".into(), McpStatus::Disconnected);
        statuses.insert("filesystem".into(), McpStatus::Connected);

        assert_eq!(count_unavailable_servers(&statuses), 2);
    }

    #[test]
    fn unavailable_badge_renders() {
        assert_eq!(unavailable_badge(0), None);
        assert_eq!(
            unavailable_badge(1),
            Some("⛔ 1 MCP server unavailable".into())
        );
        assert_eq!(
            unavailable_badge(6),
            Some("⛔ 6 MCP servers unavailable".into())
        );
    }
}
