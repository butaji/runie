//! MCP server configuration types.
//!
//! Defines the configuration schema for MCP servers that can be added to
//! `~/.runie/config.toml` or `./.runie/config.toml`.

use std::collections::HashMap;

use schemars::JsonSchema;

// ============================================================================
// Transport
// ============================================================================

/// Transport type for MCP server communication.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(JsonSchema)]
pub enum McpTransport {
    /// Stdio-based MCP server (default).
    #[default]
    Stdio,
    /// HTTP-based MCP server.
    Http,
    /// Server-Sent Events transport.
    Sse,
}

// ============================================================================
// Server
// ============================================================================

fn default_scope() -> String {
    "user".to_string()
}

/// An MCP server configuration entry.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[derive(JsonSchema)]
pub struct McpServer {
    /// Transport type: stdio, http, or sse.
    #[serde(default)]
    pub transport: McpTransport,
    /// Command and arguments for stdio transport.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    /// URL for http/sse transports.
    pub url: Option<String>,
    /// HTTP headers for http/sse transports.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
    /// Scope: user or project.
    #[serde(default = "default_scope")]
    pub scope: String,
}

// ============================================================================
// Section
// ============================================================================

/// MCP server configuration section.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct McpSection {
    /// Named MCP server configurations.
    pub servers: HashMap<String, McpServer>,
}

impl McpSection {
    /// Create an empty MCP section.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any servers are configured.
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }

    /// Get a server by name.
    pub fn get(&self, name: &str) -> Option<&McpServer> {
        self.servers.get(name)
    }

    /// Insert or update a server.
    pub fn insert(&mut self, name: String, server: McpServer) {
        self.servers.insert(name, server);
    }

    /// Remove a server by name.
    pub fn remove(&mut self, name: &str) -> Option<McpServer> {
        self.servers.remove(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_transport_serialization() {
        let stdio = McpTransport::Stdio;
        let http = McpTransport::Http;
        let sse = McpTransport::Sse;

        let stdio_str = serde_json::to_string(&stdio).unwrap();
        assert_eq!(stdio_str, "\"stdio\"");

        let http_str = serde_json::to_string(&http).unwrap();
        assert_eq!(http_str, "\"http\"");

        let sse_str = serde_json::to_string(&sse).unwrap();
        assert_eq!(sse_str, "\"sse\"");
    }

    #[test]
    fn mcp_server_stdio() {
        let server = McpServer {
            transport: McpTransport::Stdio,
            command: vec!["npx".to_string(), "-y".to_string(), "@mcp/server".to_string()],
            url: None,
            headers: HashMap::new(),
            scope: "user".to_string(),
        };

        let json = serde_json::to_string_pretty(&server).unwrap();
        assert!(json.contains("\"command\""));
        assert!(json.contains("npx"));

        let back: McpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.command, server.command);
        assert_eq!(back.scope, "user");
    }

    #[test]
    fn mcp_server_http_with_headers() {
        let server = McpServer {
            transport: McpTransport::Http,
            command: Vec::new(),
            url: Some("https://api.example.com/mcp".to_string()),
            headers: [("Authorization".to_string(), "Bearer token".to_string())]
                .into_iter()
                .collect(),
            scope: "project".to_string(),
        };

        let json = serde_json::to_string_pretty(&server).unwrap();
        assert!(json.contains("Authorization"));
        assert!(json.contains("Bearer token"));

        let back: McpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.transport, McpTransport::Http);
        assert_eq!(
            back.headers.get("Authorization"),
            Some(&"Bearer token".to_string())
        );
    }

    #[test]
    fn mcp_section_is_empty() {
        let section = McpSection::default();
        assert!(section.is_empty());
    }

    #[test]
    fn mcp_section_insert_get_remove() {
        let mut section = McpSection::new();
        assert!(section.is_empty());

        let server = McpServer {
            transport: McpTransport::Stdio,
            command: vec!["npx".to_string()],
            url: None,
            headers: HashMap::new(),
            scope: "user".to_string(),
        };

        section.insert("test".to_string(), server.clone());
        assert!(!section.is_empty());
        assert_eq!(section.get("test"), Some(&server));

        let removed = section.remove("test");
        assert_eq!(removed, Some(server));
        assert!(section.is_empty());
    }
}
