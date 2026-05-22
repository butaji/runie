use std::sync::Arc;
use std::pin::Pin;

/// Handler for executing a tool with JSON arguments, returning a future that resolves to a result string or error.
pub type ToolHandler = Arc<dyn Fn(serde_json::Value) -> Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct AgentTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub handler: Option<ToolHandler>,
}

impl std::fmt::Debug for AgentTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("parameters", &self.parameters)
            .field("handler", &if self.handler.is_some() { "Some(...)" } else { "None" })
            .finish()
    }
}

impl AgentTool {
    pub fn new(name: String, description: String, parameters: serde_json::Value) -> Self {
        Self {
            name,
            description,
            parameters,
            handler: None,
        }
    }

    pub fn with_handler(mut self, handler: ToolHandler) -> Self {
        self.handler = Some(handler);
        self
    }
}
