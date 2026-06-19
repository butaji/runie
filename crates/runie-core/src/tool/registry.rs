//! [`Tool`] trait and [`ToolRegistry`].

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::{ToolContext, ToolOutput};

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    fn is_read_only(&self) -> bool {
        false
    }
    fn requires_approval(&self, _input: &Value) -> bool {
        true
    }
    async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput>;
}

#[derive(Clone)]
pub struct ToolRegistry {
    pub(crate) tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn list(&self) -> Vec<&Arc<dyn Tool>> {
        self.tools.values().collect()
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    pub fn schemas(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.input_schema(),
                })
            })
            .collect()
    }

    /// Convert the registry into OpenAI function-style tool definitions.
    pub fn to_openai_functions(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.input_schema(),
                    }
                })
            })
            .collect()
    }

    /// Return a registry containing only tools that are safe in read-only mode.
    pub fn read_only_subset(&self) -> Self {
        const WRITE_TOOLS: &[&str] = &["write_file", "edit_file", "bash"];
        let mut filtered = Self::new();
        for tool in self.tools.values() {
            if !WRITE_TOOLS.contains(&tool.name()) {
                filtered.register(tool.clone());
            }
        }
        filtered
    }

    /// Return a registry containing only the named tools.
    pub fn filtered(&self, names: &[String]) -> Self {
        let mut filtered = Self::new();
        for name in names {
            if let Some(tool) = self.tools.get(name) {
                filtered.register(tool.clone());
            }
        }
        filtered
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyTool;

    #[async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &str {
            "dummy"
        }

        fn description(&self) -> &str {
            "A dummy tool for testing."
        }

        fn input_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "value": { "type": "string" }
                }
            })
        }

        async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
            anyhow::bail!("not implemented")
        }
    }

    #[test]
    fn to_openai_functions_wraps_schemas() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(DummyTool));
        let functions = registry.to_openai_functions();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0]["type"], "function");
        assert_eq!(functions[0]["function"]["name"], "dummy");
        assert_eq!(
            functions[0]["function"]["parameters"]["properties"]["value"]["type"],
            "string"
        );
    }
}
