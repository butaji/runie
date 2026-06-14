use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_dir: PathBuf,
    pub env: HashMap<String, String>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_default(),
            env: std::env::vars().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    Success,
    Error,
    Blocked,
}

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub content: String,
    pub bytes_transferred: Option<u64>,
    pub duration: Duration,
    pub status: ToolStatus,
}

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

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
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
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }
        fn description(&self) -> &str {
            "A test tool"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            })
        }
        async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
            Ok(ToolOutput {
                content: "done".to_string(),
                bytes_transferred: None,
                duration: Duration::from_millis(1),
                status: ToolStatus::Success,
            })
        }
    }

    struct ReadOnlyTestTool;

    #[async_trait]
    impl Tool for ReadOnlyTestTool {
        fn name(&self) -> &str {
            "read_only_test"
        }
        fn description(&self) -> &str {
            "A read-only tool"
        }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object", "properties": {}})
        }
        fn is_read_only(&self) -> bool {
            true
        }
        async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
            Ok(ToolOutput {
                content: "read done".to_string(),
                bytes_transferred: None,
                duration: Duration::from_millis(1),
                status: ToolStatus::Success,
            })
        }
    }

    #[tokio::test]
    async fn registry_registers_and_retrieves_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestTool));
        let tool = registry.get("test_tool");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "test_tool");
    }

    #[tokio::test]
    async fn registry_schemas_include_name_and_description() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestTool));
        let schemas = registry.schemas();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0]["name"], "test_tool");
        assert_eq!(schemas[0]["description"], "A test tool");
        assert!(schemas[0]["input_schema"].is_object());
    }

    #[tokio::test]
    async fn read_only_tool_returns_true() {
        let ro = ReadOnlyTestTool;
        assert!(ro.is_read_only());
        let mut rw = TestTool;
        assert!(!rw.is_read_only());
    }

    #[tokio::test]
    async fn tool_output_records_bytes_and_duration() {
        let tool = TestTool;
        let ctx = ToolContext::default();
        let output = tool
            .call(serde_json::json!({"input": "test"}), &ctx)
            .await
            .unwrap();
        assert!(output.duration.as_millis() >= 1);
        assert_eq!(output.status, ToolStatus::Success);
        assert_eq!(output.content, "done");
    }
}
