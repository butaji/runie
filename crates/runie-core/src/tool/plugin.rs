//! Plugin architecture for tool registration (from Grok Build)

use std::sync::OnceLock;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// A tool pack is a function that registers tools
pub type ToolPack = fn(&mut ToolRegistryBuilder);

/// Global registry of tool packs
static TOOL_PACKS: OnceLock<Mutex<Vec<ToolPack>>> = OnceLock::new();

/// Get the global tool packs registry
pub fn tool_packs() -> &'static Mutex<Vec<ToolPack>> {
    TOOL_PACKS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Register a new tool pack
pub fn register_tool_pack(pack: ToolPack) {
    tool_packs().lock().push(pack);
}

/// Tool registry builder for plugin use
#[derive(Default)]
pub struct ToolRegistryBuilder {
    tools: Vec<ToolDef>,
    servers: Vec<ServerDef>,
}

impl ToolRegistryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            servers: Vec::new(),
        }
    }

    /// Register a new tool
    pub fn register(
        &mut self,
        name: &str,
        description: &str,
        handler: impl Fn(serde_json::Value) -> anyhow::Result<serde_json::Value> + Send + Sync + 'static,
    ) {
        self.tools.push(ToolDef {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({}),
            output_schema: None,
            handler: Box::new(handler),
            permissions: ToolPermissions::default(),
        });
    }

    /// Register a tool with full schema
    pub fn register_with_schema(
        &mut self,
        name: &str,
        description: &str,
        input_schema: serde_json::Value,
        output_schema: Option<serde_json::Value>,
        handler: impl Fn(serde_json::Value) -> anyhow::Result<serde_json::Value> + Send + Sync + 'static,
    ) {
        self.tools.push(ToolDef {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
            output_schema,
            handler: Box::new(handler),
            permissions: ToolPermissions::default(),
        });
    }

    /// Register a tool with permissions
    pub fn register_with_permissions(
        &mut self,
        name: &str,
        description: &str,
        permissions: ToolPermissions,
        handler: impl Fn(serde_json::Value) -> anyhow::Result<serde_json::Value> + Send + Sync + 'static,
    ) {
        self.tools.push(ToolDef {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({}),
            output_schema: None,
            handler: Box::new(handler),
            permissions,
        });
    }

    /// Register an MCP server
    pub fn register_server(&mut self, server: ServerDef) {
        self.servers.push(server);
    }

    /// Build the registry
    pub fn build(self) -> (Vec<ToolDef>, Vec<ServerDef>) {
        (self.tools, self.servers)
    }

    /// Get count of registered tools
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

/// A registered tool definition
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub handler: Box<dyn Fn(serde_json::Value) -> anyhow::Result<serde_json::Value> + Send + Sync>,
    pub permissions: ToolPermissions,
}

impl std::fmt::Debug for ToolDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolDef")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_schema", &self.input_schema)
            .finish()
    }
}

/// Server definition
#[derive(Debug, Clone)]
pub struct ServerDef {
    pub name: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub transport: ServerTransport,
    pub tools: Vec<ToolDef>,
}

impl ServerDef {
    /// Create a command-based server
    pub fn command(name: &str, command: &str, args: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            command: Some(command.to_string()),
            args,
            env: Vec::new(),
            transport: ServerTransport::Stdio,
            tools: Vec::new(),
        }
    }

    /// Create an MCP URL server
    pub fn url(name: &str, url: &str) -> Self {
        Self {
            name: name.to_string(),
            command: None,
            args: Vec::new(),
            env: Vec::new(),
            transport: ServerTransport::Http { url: url.to_string() },
            tools: Vec::new(),
        }
    }

    /// Add environment variable
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_string(), value.to_string()));
        self
    }
}

/// Server transport type
#[derive(Debug, Clone)]
pub enum ServerTransport {
    Stdio,
    Http { url: String },
    WebSocket { url: String },
}

/// Tool permissions
#[derive(Debug, Clone, Default)]
pub struct ToolPermissions {
    pub read: bool,
    pub write: bool,
    pub network: bool,
    pub exec: bool,
}

impl ToolPermissions {
    /// Read-only permissions
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            network: false,
            exec: false,
        }
    }

    /// Read-write permissions
    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            network: false,
            exec: false,
        }
    }

    /// All permissions
    pub fn all() -> Self {
        Self {
            read: true,
            write: true,
            network: true,
            exec: true,
        }
    }

    /// No permissions
    pub fn none() -> Self {
        Self {
            read: false,
            write: false,
            network: false,
            exec: false,
        }
    }
}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub cwd: std::path::PathBuf,
    pub env: std::collections::HashMap<String, String>,
    pub user_id: Option<String>,
}

impl ToolContext {
    /// Create a new context
    pub fn new(session_id: String, cwd: std::path::PathBuf) -> Self {
        Self {
            session_id,
            cwd,
            env: std::env::vars().collect(),
            user_id: None,
        }
    }

    /// Get an environment variable
    pub fn env(&self, key: &str) -> Option<&str> {
        self.env.get(key).map(|s| s.as_str())
    }
}

/// Tool execution result
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl ToolResult {
    /// Create a successful result
    pub fn ok(output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            success: true,
            output,
            error: None,
            duration_ms,
        }
    }

    /// Create an error result
    pub fn error(msg: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(msg.into()),
            duration_ms,
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> serde_json::Value {
        if self.success {
            self.output.clone()
        } else {
            serde_json::json!({
                "error": self.error,
            })
        }
    }
}

/// Tool registry for looking up and executing tools
#[derive(Default)]
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, ToolDef>,
    servers: std::collections::HashMap<String, ServerDef>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize from registered tool packs
    pub fn from_packs() -> Self {
        let packs = tool_packs().lock();
        let mut builder = ToolRegistryBuilder::new();

        for pack in packs.iter() {
            pack(&mut builder);
        }

        let (tools, servers) = builder.build();
        let mut registry = Self::new();

        for tool in tools {
            registry.register_tool(tool);
        }

        for server in servers {
            registry.register_server(server);
        }

        registry
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: ToolDef) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Register a server
    pub fn register_server(&mut self, server: ServerDef) {
        self.servers.insert(server.name.clone(), server);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&ToolDef> {
        self.tools.get(name)
    }

    /// Execute a tool by name
    pub fn execute(
        &self,
        name: &str,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> anyhow::Result<ToolResult> {
        let start = std::time::Instant::now();

        let tool = self.tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;

        let result = (tool.handler)(input);

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => Ok(ToolResult::ok(output, duration_ms)),
            Err(e) => Ok(ToolResult::error(e.to_string(), duration_ms)),
        }
    }

    /// List all tool names
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// List all server names
    pub fn server_names(&self) -> Vec<&str> {
        self.servers.keys().map(|s| s.as_str()).collect()
    }

    /// Get tool count
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

/// Plugin trait for tool pack implementations
pub trait ToolPlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;

    /// Register tools with the builder
    fn register(&self, builder: &mut ToolRegistryBuilder);
}

/// Macro to create a tool pack from functions
#[macro_export]
macro_rules! tool_pack {
    ($name:ident => { $($tool_name:expr => $handler:expr),* $(,)? }) => {
        fn $name(builder: &mut $crate::tool::plugin::ToolRegistryBuilder) {
            $(builder.register($tool_name, "", $handler);)*
        }
    };
}

/// Example: Create a simple tool pack
#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_handler(input: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        Ok(input)
    }

    #[test]
    fn test_tool_registry_builder() {
        let mut builder = ToolRegistryBuilder::new();
        builder.register("echo", "Echo back the input", dummy_handler);
        builder.register("noop", "Do nothing", |_| Ok(serde_json::Value::Null));

        assert_eq!(builder.tool_count(), 2);
    }

    #[test]
    fn test_tool_pack_registration() {
        let mut builder = ToolRegistryBuilder::new();

        register_tool_pack(|b| {
            b.register("test", "A test tool", dummy_handler);
        });

        let (tools, _) = builder.build();
        assert_eq!(tools.len(), 1);
    }

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register_tool(ToolDef {
            name: "hello".to_string(),
            description: "Say hello".to_string(),
            input_schema: serde_json::json!({}),
            output_schema: None,
            handler: Box::new(|_| Ok(serde_json::json!("Hello!"))),
            permissions: ToolPermissions::default(),
        });

        assert!(registry.has_tool("hello"));
        assert_eq!(registry.tool_count(), 1);

        let result = registry.execute("hello", serde_json::Value::Null, &ToolContext::new(
            "test".to_string(),
            std::path::PathBuf::from("."),
        )).unwrap();

        assert!(result.success);
    }

    #[test]
    fn test_tool_result() {
        let ok = ToolResult::ok(serde_json::json!({"result": "ok"}), 100);
        assert!(ok.success);
        assert!(ok.error.is_none());

        let err = ToolResult::error("Something went wrong", 50);
        assert!(!err.success);
        assert!(err.error.is_some());
    }
}
