use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpStatus {
    Connected,
    Disconnected,
    Unavailable,
}

pub fn load_mcp_servers(_path: &std::path::Path) -> Vec<McpServerConfig> {
    Vec::new()
}

pub fn generate_default_statuses(servers: &[McpServerConfig]) -> HashMap<String, McpStatus> {
    servers.iter().map(|s| (s.name.clone(), McpStatus::Unavailable)).collect()
}

pub fn mcp_config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("runie")
        .join("mcp.toml")
}
