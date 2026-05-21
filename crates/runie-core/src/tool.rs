use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolOutput {
    pub content: String,
    pub metadata: serde_json::Value,
    pub terminate: bool,
}

impl Default for ToolOutput {
    fn default() -> Self {
        Self {
            content: String::new(),
            metadata: serde_json::Value::Null,
            terminate: false,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ToolError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("not found: {0}")]
    NotFound(String),
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> ToolSchema;
    async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput, ToolError>;
}
