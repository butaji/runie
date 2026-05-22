use runie_core::{ToolCall, ToolOutput, ToolError};
use runie_tools::ToolRegistry;
use crate::{Hook, config::ToolExecutionMode};
use std::sync::Arc;

pub struct ToolExecutor {
    pub registry: Arc<ToolRegistry>,
    pub hooks: Vec<Arc<dyn Hook>>,
    pub mode: ToolExecutionMode,
}

impl ToolExecutor {
    pub fn new(registry: Arc<ToolRegistry>, hooks: Vec<Arc<dyn Hook>>, mode: ToolExecutionMode) -> Self {
        Self { registry, hooks, mode }
    }

    pub async fn execute(&self, tool_calls: Vec<ToolCall>) -> Vec<(String, Result<ToolOutput, String>)> {
        match self.mode {
            ToolExecutionMode::Parallel => self.execute_parallel(tool_calls).await,
            ToolExecutionMode::Sequential => self.execute_sequential(tool_calls).await,
        }
    }

    pub fn schemas(&self) -> Vec<runie_core::ToolSchema> {
        self.registry.schemas()
    }

    async fn execute_sequential(&self, tool_calls: Vec<ToolCall>) -> Vec<(String, Result<ToolOutput, String>)> {
        let mut results = Vec::new();
        for call in tool_calls {
            results.push(self.execute_single(call).await);
        }
        results
    }

    async fn execute_parallel(&self, tool_calls: Vec<ToolCall>) -> Vec<(String, Result<ToolOutput, String>)> {
        let futures: Vec<_> = tool_calls.into_iter()
            .map(|call| self.execute_single(call))
            .collect();
        futures::future::join_all(futures).await
    }

    async fn execute_single(&self, tool_call: ToolCall) -> (String, Result<ToolOutput, String>) {
        let id = tool_call.id.clone();

        let result = match self.registry.get(&tool_call.name) {
            Some(tool) => tool.execute(tool_call.arguments.clone()).await,
            None => Err(ToolError::NotFound(tool_call.name)),
        };

        match result {
            Ok(output) => (id, Ok(output)),
            Err(e) => (id, Err(e.to_string())),
        }
    }
}
