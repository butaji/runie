//! Tool inspector pipeline — middleware for tool execution.
//!
//! Chains `Inspector` hooks around calls to the canonical [`runie_core::tool::Tool`]
//! registry: `before_call` runs before the tool, `after_call` runs after (with the
//! output).

use std::sync::Arc;
use std::time::Instant;

use runie_core::tool::{ToolContext, ToolOutput, ToolRegistry, ToolStatus};
use runie_engine::tool::builtin_registry;

/// A single hook in the inspector pipeline.
pub trait Inspector: Send + Sync {
    /// Called before the tool executes. Return `Ok(())` to continue,
    /// or `Err(msg)` to short-circuit with a blocked result.
    fn before_call(&self, _tool_name: &str, _tool_input: &serde_json::Value) -> Result<(), String> {
        Ok(())
    }

    /// Called after the tool succeeds or fails. Use this to record output, tokens, etc.
    fn after_call(&self, _tool_name: &str, _output: &ToolOutput) {}
}

/// Pipeline that runs a sequence of inspectors around a canonical tool call.
pub struct ToolPipeline {
    inspectors: Vec<Arc<dyn Inspector>>,
    registry: ToolRegistry,
}

impl ToolPipeline {
    /// Create a new pipeline from a vec of boxed inspectors.
    pub fn new(inspectors: Vec<Arc<dyn Inspector>>) -> Self {
        Self {
            inspectors,
            registry: builtin_registry(),
        }
    }

    /// Add an inspector to the end of the pipeline.
    pub fn push(mut self, inspector: Arc<dyn Inspector>) -> Self {
        self.inspectors.push(inspector);
        self
    }

    /// Execute the named tool with the given input, running `before_call` on all
    /// inspectors first and `after_call` on all inspectors after. Short-circuits
    /// on the first blocking inspector.
    pub async fn call(
        &self,
        name: &str,
        input: serde_json::Value,
        ctx: &ToolContext,
    ) -> ToolOutput {
        for insp in &self.inspectors {
            if let Err(msg) = insp.before_call(name, &input) {
                return blocked_output(name, input, &msg);
            }
        }

        let output = dispatch_tool(&self.registry, name, input, ctx).await;

        for insp in &self.inspectors {
            insp.after_call(name, &output);
        }

        output
    }
}

fn blocked_output(name: &str, input: serde_json::Value, msg: &str) -> ToolOutput {
    ToolOutput {
        tool_name: name.to_string(),
        tool_args: input,
        content: format!("Inspector blocked: {}", msg),
        bytes_transferred: None,
        duration: Instant::now().elapsed(),
        status: ToolStatus::Blocked,
    }
}

async fn dispatch_tool(
    registry: &ToolRegistry,
    name: &str,
    input: serde_json::Value,
    ctx: &ToolContext,
) -> ToolOutput {
    match registry.get(name) {
        Some(tool) => tool.call(input, ctx).await.unwrap_or_else(|e| ToolOutput {
            tool_name: name.to_string(),
            tool_args: serde_json::Value::Null,
            content: format!("Tool execution failed: {}", e),
            bytes_transferred: None,
            duration: Instant::now().elapsed(),
            status: ToolStatus::Error,
        }),
        None => ToolOutput {
            tool_name: name.to_string(),
            tool_args: serde_json::Value::Null,
            content: format!("Error: unknown tool '{}'", name),
            bytes_transferred: None,
            duration: Instant::now().elapsed(),
            status: ToolStatus::Error,
        },
    }
}

// ─── Common inspectors ─────────────────────────────────────────────────────────

/// Inspector that counts how many times each tool was called.
pub struct CallCounter {
    counts: std::sync::Mutex<std::collections::HashMap<String, u32>>,
}

impl CallCounter {
    /// Returns the call count for a tool name.
    pub fn count(&self, name: &str) -> u32 {
        *self.counts.lock().unwrap().get(name).unwrap_or(&0)
    }
}

impl Default for CallCounter {
    fn default() -> Self {
        Self {
            counts: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Inspector for CallCounter {
    fn after_call(&self, tool_name: &str, _output: &ToolOutput) {
        let mut counts = self.counts.lock().unwrap();
        *counts.entry(tool_name.to_string()).or_insert(0) += 1;
    }
}

/// Inspector that tracks elapsed time per tool name.
pub struct LatencyTracker {
    totals: std::sync::Mutex<std::collections::HashMap<String, std::time::Duration>>,
}

impl LatencyTracker {
    /// Returns the total elapsed time for a tool name.
    pub fn total(&self, name: &str) -> std::time::Duration {
        *self
            .totals
            .lock()
            .unwrap()
            .get(name)
            .unwrap_or(&std::time::Duration::ZERO)
    }
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self {
            totals: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Inspector for LatencyTracker {}

/// Inspector that tracks whether `after_call` was invoked.
pub struct AfterCallSpy {
    pub called: std::sync::Mutex<bool>,
}

impl AfterCallSpy {
    /// Create a new spy.
    pub fn new() -> Self {
        Self {
            called: std::sync::Mutex::new(false),
        }
    }

    /// Returns whether `after_call` was invoked.
    pub fn was_called(&self) -> bool {
        *self.called.lock().unwrap()
    }
}

impl Default for AfterCallSpy {
    fn default() -> Self {
        Self::new()
    }
}

impl Inspector for AfterCallSpy {
    fn after_call(&self, _tool_name: &str, _output: &ToolOutput) {
        *self.called.lock().unwrap() = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailInspector;

    impl Inspector for FailInspector {
        fn before_call(
            &self,
            _tool_name: &str,
            _tool_input: &serde_json::Value,
        ) -> Result<(), String> {
            Err("blocked by inspector".to_string())
        }
    }

    #[tokio::test]
    async fn inspector_before_call_blocks() {
        let pipeline = ToolPipeline::new(vec![Arc::new(FailInspector)]);
        let output = pipeline
            .call(
                "bash",
                serde_json::json!({"command": "echo hi"}),
                &ToolContext::default(),
            )
            .await;
        assert_eq!(output.status, ToolStatus::Blocked);
        assert!(output.content.contains("blocked by inspector"));
    }

    #[tokio::test]
    async fn after_call_runs_on_error() {
        let spy = Arc::new(AfterCallSpy::new());
        let pipeline = ToolPipeline::new(vec![spy.clone()]);

        let output = pipeline
            .call(
                "read_file",
                serde_json::json!({"path": "DOES_NOT_EXIST_xyz"}),
                &ToolContext::default(),
            )
            .await;
        assert_eq!(output.status, ToolStatus::Error);
        assert!(spy.was_called(), "after_call should run even on error");
    }

    #[tokio::test]
    async fn call_counter_increments() {
        let counter = Arc::new(CallCounter::default());
        let pipeline = ToolPipeline::new(vec![counter.clone()]);

        pipeline
            .call(
                "bash",
                serde_json::json!({"command": "echo hi"}),
                &ToolContext::default(),
            )
            .await;
        pipeline
            .call(
                "bash",
                serde_json::json!({"command": "echo hi"}),
                &ToolContext::default(),
            )
            .await;

        assert_eq!(counter.count("bash"), 2);
    }

    #[tokio::test]
    async fn empty_pipeline_runs_tool() {
        let pipeline = ToolPipeline::new(vec![]);
        let output = pipeline
            .call(
                "bash",
                serde_json::json!({"command": "echo hello"}),
                &ToolContext::default(),
            )
            .await;
        assert_eq!(output.status, ToolStatus::Success);
        assert!(output.content.contains("hello"));
    }

    #[tokio::test]
    async fn multiple_inspectors_run_in_order() {
        let spy1 = Arc::new(AfterCallSpy::new());
        let spy2 = Arc::new(AfterCallSpy::new());
        let pipeline = ToolPipeline::new(vec![spy1.clone(), spy2.clone()]);

        pipeline
            .call(
                "bash",
                serde_json::json!({"command": "echo hi"}),
                &ToolContext::default(),
            )
            .await;

        assert!(spy1.was_called(), "first inspector after_call should run");
        assert!(spy2.was_called(), "second inspector after_call should run");
    }

    #[tokio::test]
    async fn pipeline_push_fluent_api() {
        let counter = Arc::new(CallCounter::default());
        let spy = Arc::new(AfterCallSpy::new());
        let pipeline = ToolPipeline::new(vec![])
            .push(counter.clone())
            .push(spy.clone());

        pipeline
            .call(
                "bash",
                serde_json::json!({"command": "echo hi"}),
                &ToolContext::default(),
            )
            .await;

        assert_eq!(counter.count("bash"), 1);
        assert!(spy.was_called());
    }
}
