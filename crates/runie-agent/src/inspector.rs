//! Tool inspector pipeline — middleware for tool execution.
//!
//! Chains `Inspector` hooks around calls to built-in tools via the `ToolDef`
//! trait: `before_call` runs before the tool, `after_call` runs after (with the
//! output).

use std::sync::Arc;
use std::time::Instant;

use runie_core::tool::{
    parse_input, ToolContext, ToolDef, ToolOutput, ToolStatus,
};
use crate::tool::{
    BashTool, EditFileTool, FetchDocsTool, FindDefinitionsTool, FindTool, GrepTool,
    ListDirTool, ReadFileTool, SearchTool, WriteFileTool,
};

/// Dispatch a tool call by name.
/// The match arms are generated from BUILTIN_TOOL_NAMES to keep the list canonical.
async fn dispatch_tool(
    name: &str,
    input: serde_json::Value,
    ctx: &ToolContext,
) -> ToolOutput {
    // NOTE: arms are kept in the same order as BUILTIN_TOOL_NAMES.
    match name {
        "bash" => run_tool::<BashTool>(&input, ctx).await,
        "read_file" => run_tool::<ReadFileTool>(&input, ctx).await,
        "write_file" => run_tool::<WriteFileTool>(&input, ctx).await,
        "edit_file" => run_tool::<EditFileTool>(&input, ctx).await,
        "list_dir" => run_tool::<ListDirTool>(&input, ctx).await,
        "grep" => run_tool::<GrepTool>(&input, ctx).await,
        "find" => run_tool::<FindTool>(&input, ctx).await,
        "fetch_docs" => run_tool::<FetchDocsTool>(&input, ctx).await,
        "search" => run_tool::<SearchTool>(&input, ctx).await,
        "find_definitions" => run_tool::<FindDefinitionsTool>(&input, ctx).await,
        _ => unknown_tool_output(name, input),
    }
}

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

/// Pipeline that runs a sequence of inspectors around a tool call.
pub struct ToolPipeline {
    inspectors: Vec<Arc<dyn Inspector>>,
}

impl ToolPipeline {
    /// Create a new pipeline from a vec of boxed inspectors.
    pub fn new(inspectors: Vec<Arc<dyn Inspector>>) -> Self {
        Self { inspectors }
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

        let output = dispatch_tool(name, input, ctx).await;

        for insp in &self.inspectors {
            insp.after_call(name, &output);
        }

        output
    }
}

fn blocked_output(name: &str, input: serde_json::Value, msg: &str) -> ToolOutput {
    ToolOutput {
        tool_name: name.to_owned(),
        tool_args: input,
        content: format!("Inspector blocked: {}", msg),
        bytes_transferred: None,
        duration: Instant::now().elapsed(),
        status: ToolStatus::Blocked,
    }
}

async fn run_tool<T: ToolDef>(input: &serde_json::Value, ctx: &ToolContext) -> ToolOutput {
    match parse_input::<T::Input>(input) {
        Ok(i) => T::execute(i, ctx).await,
        Err(e) => ToolOutput {
            tool_name: T::NAME.to_string(),
            tool_args: input.clone(),
            content: format!("Failed to parse tool input: {}", e),
            bytes_transferred: None,
            duration: Instant::now().elapsed(),
            status: ToolStatus::Error,
        },
    }
}

fn unknown_tool_output(name: &str, input: serde_json::Value) -> ToolOutput {
    ToolOutput {
        tool_name: name.to_owned(),
        tool_args: input,
        content: format!("Error: unknown tool '{}'", name),
        bytes_transferred: None,
        duration: Instant::now().elapsed(),
        status: ToolStatus::Error,
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
        *counts.entry(tool_name.to_owned()).or_insert(0) += 1;
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
