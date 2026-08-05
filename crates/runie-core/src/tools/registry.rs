//! Tool registry: lookup by name and per-tool execution mode.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{AgentTool, ToolExecutionMode};

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn AgentTool>>,
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
        }
    }

    pub fn register(&mut self, tool: Arc<dyn AgentTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn lookup(&self, name: &str) -> Option<Arc<dyn AgentTool>> {
        self.tools.get(name).cloned()
    }

    pub fn execution_mode(&self, name: &str) -> Option<ToolExecutionMode> {
        self.tools.get(name).and_then(|t| t.execution_mode())
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentToolResult, ToolResultContent};

    struct EchoTool;
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
        r.register(Arc::new(EchoTool));
        assert!(r.lookup("echo").is_some());
        assert_eq!(r.len(), 1);
        assert_eq!(r.execution_mode("echo"), None);
    }
}
