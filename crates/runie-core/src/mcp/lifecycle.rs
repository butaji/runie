//! MCP server lifecycle management (from Grok Build)

use std::collections::HashSet;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use regex::Regex;

/// MCP server initialization state
#[derive(Debug, Clone)]
pub enum InitProgress {
    /// Initialization not started
    NotStarted,
    /// Initialization in progress
    Starting {
        handshaking: HashSet<McpServerName>,
    },
    /// Initialization complete
    Finished {
        ready: HashSet<McpServerName>,
    },
}

impl Default for InitProgress {
    fn default() -> Self {
        Self::NotStarted
    }
}

impl InitProgress {
    /// Check if initialization is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, InitProgress::Finished { .. })
    }

    /// Check if initialization is in progress
    pub fn is_in_progress(&self) -> bool {
        matches!(self, InitProgress::Starting { .. })
    }

    /// Check if initialization has not started
    pub fn is_not_started(&self) -> bool {
        matches!(self, InitProgress::NotStarted)
    }

    /// Try to start initialization for a server
    pub fn try_start(&mut self, server: McpServerName) -> bool {
        match self {
            InitProgress::NotStarted => {
                *self = InitProgress::Starting {
                    handshaking: [server].into(),
                };
                true
            }
            InitProgress::Starting { handshaking } => {
                handshaking.insert(server);
                true
            }
            InitProgress::Finished { .. } => false,
        }
    }

    /// Mark a server as ready
    pub fn finish(&mut self, server: &McpServerName) {
        match self {
            InitProgress::Starting { handshaking } => {
                handshaking.remove(server);
                if handshaking.is_empty() {
                    *self = InitProgress::Finished {
                        ready: HashSet::new(),
                    };
                }
            }
            InitProgress::Finished { ready } => {
                ready.insert(server.clone());
            }
            InitProgress::NotStarted => {}
        }
    }

    /// Get the set of servers that are ready
    pub fn ready_servers(&self) -> HashSet<&McpServerName> {
        match self {
            InitProgress::Finished { ready } => ready.iter().collect(),
            _ => HashSet::new(),
        }
    }

    /// Get the set of servers that are still handshaking
    pub fn handshaking_servers(&self) -> HashSet<&McpServerName> {
        match self {
            InitProgress::Starting { handshaking } => handshaking.iter().collect(),
            _ => HashSet::new(),
        }
    }
}

/// MCP server name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct McpServerName(pub String);

impl McpServerName {
    /// Create a new server name
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the underlying string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parse from a qualified name (e.g., "server__tool" -> "server")
    pub fn from_qualified(qualified: &str) -> Option<Self> {
        qualified.split_once("__").map(|(s, _)| Self(s.to_string()))
    }
}

impl std::fmt::Display for McpServerName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for McpServerName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for McpServerName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Intentional shutdown tracker
#[derive(Default)]
pub struct ShutdownState {
    shutting_down: HashSet<McpServerName>,
    restarting: HashSet<McpServerName>,
}

impl ShutdownState {
    /// Create a new shutdown state
    pub fn new() -> Self {
        Self {
            shutting_down: HashSet::new(),
            restarting: HashSet::new(),
        }
    }

    /// Mark a server as shutting down
    pub fn mark_shutting_down(&mut self, server: &McpServerName) {
        self.shutting_down.insert(server.clone());
        self.restarting.remove(server);
    }

    /// Mark a server as restarting
    pub fn mark_restarting(&mut self, server: &McpServerName) {
        self.restarting.insert(server.clone());
        self.shutting_down.remove(server);
    }

    /// Check if a server should skip automatic restart
    pub fn should_skip_restart(&self, server: &McpServerName) -> bool {
        self.shutting_down.contains(server) || self.restarting.contains(server)
    }

    /// Check if a server is intentionally shutting down
    pub fn is_shutting_down(&self, server: &McpServerName) -> bool {
        self.shutting_down.contains(server)
    }

    /// Check if a server is restarting
    pub fn is_restarting(&self, server: &McpServerName) -> bool {
        self.restarting.contains(server)
    }

    /// Clear shutdown state for a server
    pub fn clear(&mut self, server: &McpServerName) {
        self.shutting_down.remove(server);
        self.restarting.remove(server);
    }

    /// Clear all shutdown state
    pub fn clear_all(&mut self) {
        self.shutting_down.clear();
        self.restarting.clear();
    }
}

/// Event coalescing window in milliseconds
pub const COALESCE_WINDOW_MS: u64 = 50;

/// Event coalescing state
#[derive(Debug, Clone, Default)]
pub struct EventCoalescer {
    pending: HashSet<McpEvent>,
    last_flush: Instant,
}

impl EventCoalescer {
    /// Create a new coalescer
    pub fn new() -> Self {
        Self {
            pending: HashSet::new(),
            last_flush: Instant::now(),
        }
    }

    /// Add an event
    pub fn push(&mut self, event: McpEvent) {
        self.pending.insert(event);
    }

    /// Check if we should flush
    pub fn should_flush(&self) -> bool {
        self.pending.is_empty()
            || self.last_flush.elapsed() > Duration::from_millis(COALESCE_WINDOW_MS)
    }

    /// Flush pending events
    pub fn flush(&mut self) -> Vec<McpEvent> {
        if !self.should_flush() {
            return Vec::new();
        }

        let events: Vec<_> = self.pending.drain().collect();
        self.last_flush = Instant::now();
        events
    }

    /// Get pending event count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

/// MCP event types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum McpEvent {
    ToolsChanged,
    ResourcesChanged,
    PromptsChanged,
    ServerReady(String),
    ServerDisconnected(String),
}

/// Tool name validation regex
static TOOL_NAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_-]{0,63}$").unwrap()
});

/// Validate MCP tool name format (server__tool or just tool)
pub fn validate_mcp_tool_name(name: &str) -> bool {
    if let Some((_, tool)) = name.split_once("__") {
        TOOL_NAME_REGEX.is_match(tool)
    } else {
        TOOL_NAME_REGEX.is_match(name)
    }
}

/// Split qualified tool name into server and tool
pub fn split_mcp_tool_name(name: &str) -> Option<(&str, &str)> {
    name.split_once("__")
}

/// Parse server name from qualified tool name
pub fn parse_server_from_tool(name: &str) -> Option<McpServerName> {
    split_mcp_tool_name(name).map(|(server, _)| McpServerName(server.to_string()))
}

/// MCP server state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    /// Server is starting up
    Starting,
    /// Server is handshaking/initializing
    Handshaking,
    /// Server is running and ready
    Running,
    /// Server is shutting down
    ShuttingDown,
    /// Server has been stopped
    Stopped,
    /// Server crashed
    Crashed,
}

impl ServerState {
    /// Check if the server is alive
    pub fn is_alive(&self) -> bool {
        matches!(self, ServerState::Running | ServerState::Handshaking)
    }

    /// Check if the server can be restarted
    pub fn can_restart(&self) -> bool {
        matches!(self, ServerState::Stopped | ServerState::Crashed)
    }

    /// Get a human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            ServerState::Starting => "Starting",
            ServerState::Handshaking => "Connecting",
            ServerState::Running => "Ready",
            ServerState::ShuttingDown => "Stopping",
            ServerState::Stopped => "Stopped",
            ServerState::Crashed => "Crashed",
        }
    }
}

/// Server lifecycle tracker
#[derive(Debug, Clone)]
pub struct ServerLifecycle {
    pub name: McpServerName,
    pub state: ServerState,
    pub started_at: Option<Instant>,
    pub last_error: Option<String>,
    pub restart_count: u32,
}

impl ServerLifecycle {
    /// Create a new lifecycle tracker
    pub fn new(name: McpServerName) -> Self {
        Self {
            name,
            state: ServerState::Starting,
            started_at: None,
            last_error: None,
            restart_count: 0,
        }
    }

    /// Mark as started
    pub fn started(&mut self) {
        self.state = ServerState::Handshaking;
        self.started_at = Some(Instant::now());
    }

    /// Mark as handshaking complete
    pub fn handshaken(&mut self) {
        self.state = ServerState::Running;
    }

    /// Mark as shutting down
    pub fn shutting_down(&mut self) {
        self.state = ServerState::ShuttingDown;
    }

    /// Mark as stopped
    pub fn stopped(&mut self) {
        self.state = ServerState::Stopped;
        self.started_at = None;
    }

    /// Mark as crashed
    pub fn crashed(&mut self, error: impl Into<String>) {
        self.state = ServerState::Crashed;
        self.last_error = Some(error.into());
    }

    /// Prepare to restart
    pub fn prepare_restart(&mut self) {
        self.restart_count += 1;
        self.state = ServerState::Starting;
        self.last_error = None;
    }

    /// Get uptime if running
    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|started| started.elapsed())
    }

    /// Check if we've exceeded restart limit
    pub fn exceeded_restart_limit(&self, limit: u32) -> bool {
        self.restart_count >= limit
    }
}

/// MCP protocol message types
#[derive(Debug, Clone)]
pub enum McpMessage {
    /// Initialize request
    Initialize { protocol_version: String, capabilities: serde_json::Value },
    /// Initialize response
    Initialized,
    /// Tool call request
    CallTool { name: String, arguments: serde_json::Value },
    /// Tool call response
    ToolResult { content: serde_json::Value },
    /// List tools request
    ListTools,
    /// List tools response
    ToolsList { tools: Vec<McpTool> },
    /// Resource list request
    ListResources,
    /// Resource subscription
    SubscribeResource { uri: String },
    /// Cancelled notification
    Cancelled { reason: Option<String> },
    /// Progress notification
    Progress { progress: f64, total: Option<f64> },
}

/// MCP tool definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
}

impl McpTool {
    /// Get the qualified name (server__tool)
    pub fn qualified_name(&self, server: &McpServerName) -> String {
        format!("{}___{}", server.as_str(), self.name)
    }

    /// Validate the tool
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Tool name cannot be empty".to_string());
        }

        if !validate_mcp_tool_name(&self.name) {
            return Err(format!("Invalid tool name: {}", self.name));
        }

        // Validate input schema is valid JSON Schema
        if !serde_json::Value::is_object(&self.input_schema) {
            return Err("Input schema must be a JSON object".to_string());
        }

        Ok(())
    }
}

/// Protocol version constants
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_progress() {
        let mut progress = InitProgress::NotStarted;
        assert!(progress.is_not_started());
        assert!(!progress.is_complete());

        let server = McpServerName::new("test");
        assert!(progress.try_start(server.clone()));
        assert!(progress.is_in_progress());

        progress.finish(&server);
        assert!(progress.is_complete());
    }

    #[test]
    fn test_mcp_server_name() {
        let name = McpServerName::new("my-server");
        assert_eq!(name.as_str(), "my-server");
        assert_eq!(name.to_string(), "my-server");

        let qualified = McpServerName::from_qualified("my-server__tool");
        assert_eq!(qualified, Some(McpServerName::new("my-server")));
    }

    #[test]
    fn test_shutdown_state() {
        let mut state = ShutdownState::new();
        let server = McpServerName::new("test");

        assert!(!state.should_skip_restart(&server));

        state.mark_shutting_down(&server);
        assert!(state.should_skip_restart(&server));
        assert!(state.is_shutting_down(&server));

        state.mark_restarting(&server);
        assert!(state.is_restarting(&server));

        state.clear(&server);
        assert!(!state.should_skip_restart(&server));
    }

    #[test]
    fn test_event_coalescer() {
        let mut coalescer = EventCoalescer::new();

        coalescer.push(McpEvent::ToolsChanged);
        coalescer.push(McpEvent::ResourcesChanged);
        assert_eq!(coalescer.pending_count(), 2);

        // Should not flush immediately
        assert!(!coalescer.should_flush());

        // Simulate time passing
        std::thread::sleep(Duration::from_millis(COALESCE_WINDOW_MS + 1));

        // Now should flush
        assert!(coalescer.should_flush());
        let events = coalescer.flush();
        assert_eq!(events.len(), 2);
        assert_eq!(coalescer.pending_count(), 0);
    }

    #[test]
    fn test_validate_tool_name() {
        assert!(validate_mcp_tool_name("read_file"));
        assert!(validate_mcp_tool_name("server__read_file"));
        assert!(validate_mcp_tool_name("_private_tool"));
        assert!(validate_mcp_tool_name("tool123"));
        assert!(validate_mcp_tool_name("my-tool"));
        assert!(validate_mcp_tool_name("my_tool"));

        assert!(!validate_mcp_tool_name(""));
        assert!(!validate_mcp_tool_name("123tool"));
        assert!(!validate_mcp_tool_name("tool-name!"));
        assert!(!validate_mcp_tool_name("a".repeat(100).as_str()));
    }

    #[test]
    fn test_server_lifecycle() {
        let mut lifecycle = ServerLifecycle::new(McpServerName::new("test"));

        assert!(!lifecycle.state.is_alive());

        lifecycle.started();
        assert!(lifecycle.state.is_alive());
        assert!(lifecycle.uptime().is_some());

        lifecycle.handshaken();
        assert!(lifecycle.state.is_alive());

        lifecycle.crashed("Connection refused");
        assert!(!lifecycle.state.is_alive());
        assert_eq!(lifecycle.last_error, Some("Connection refused".to_string()));

        lifecycle.prepare_restart();
        assert_eq!(lifecycle.restart_count, 1);
        assert!(lifecycle.state.can_restart());
    }

    #[test]
    fn test_mcp_tool() {
        let tool = McpTool {
            name: "read_file".to_string(),
            description: Some("Read a file".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                }
            }),
            output_schema: None,
        };

        assert!(tool.validate().is_ok());
        assert_eq!(tool.qualified_name(&McpServerName::new("fs")), "fs___read_file");

        let invalid_tool = McpTool {
            name: "".to_string(),
            description: None,
            input_schema: serde_json::json!({}),
            output_schema: None,
        };
        assert!(invalid_tool.validate().is_err());
    }
}
