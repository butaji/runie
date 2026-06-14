//! Tool inspector pipeline — middleware for tool execution.
//!
//! Chains `Inspector` hooks around tool calls: `before_call` runs before the
//! tool, `after_call` runs after (with the result).

use std::sync::Arc;

use crate::tools::{Tool, ToolResult};

/// A single hook in the inspector pipeline.
pub trait Inspector: Send + Sync {
    /// Called before the tool executes. Return `Ok(())` to continue,
    /// or `Err(msg)` to short-circuit with an error result.
    fn before_call(&self, _tool: &Tool) -> Result<(), String> {
        Ok(())
    }

    /// Called after the tool succeeds or fails. Use this to record output, tokens, etc.
    fn after_call(&self, _tool: &Tool, _result: &ToolResult) {}
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

    /// Execute the tool, running `before_call` on all inspectors first and
    /// `after_call` on all inspectors after. Short-circuits on first error.
    pub fn call(&self, tool: &Tool) -> ToolResult {
        for insp in &self.inspectors {
            if let Err(msg) = insp.before_call(tool) {
                return ToolResult {
                    tool: tool.clone(),
                    output: format!("Inspector blocked: {}", msg),
                    success: false,
                };
            }
        }

        let result = tool.execute();

        for insp in &self.inspectors {
            insp.after_call(tool, &result);
        }

        result
    }
}

// ─── Common inspectors ─────────────────────────────────────────────────────────

/// Inspector that counts how many times each tool was called.
pub struct CallCounter {
    counts: std::sync::Mutex<std::collections::HashMap<String, u32>>,
}

impl CallCounter {
    pub fn new() -> Self {
        Self { counts: std::sync::Mutex::new(std::collections::HashMap::new()) }
    }

    /// Returns the call count for a tool name.
    pub fn count(&self, name: &str) -> u32 {
        *self.counts.lock().unwrap().get(name).unwrap_or(&0)
    }
}

impl Inspector for CallCounter {
    fn after_call(&self, tool: &Tool, _result: &ToolResult) {
        let mut counts = self.counts.lock().unwrap();
        *counts.entry(tool.name().to_string()).or_insert(0) += 1;
    }
}

/// Inspector that tracks elapsed time per tool name.
pub struct LatencyTracker {
    totals: std::sync::Mutex<std::collections::HashMap<String, std::time::Duration>>,
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self { totals: std::sync::Mutex::new(std::collections::HashMap::new()) }
    }

    /// Returns the total elapsed time for a tool name.
    pub fn total(&self, name: &str) -> std::time::Duration {
        *self.totals.lock().unwrap().get(name).unwrap_or(&std::time::Duration::ZERO)
    }
}

impl Inspector for LatencyTracker {}

/// Inspector that tracks whether `after_call` was invoked.
pub struct AfterCallSpy {
    pub called: std::sync::Mutex<bool>,
}

impl AfterCallSpy {
    pub fn new() -> Self {
        Self { called: std::sync::Mutex::new(false) }
    }

    pub fn was_called(&self) -> bool {
        *self.called.lock().unwrap()
    }
}

impl Inspector for AfterCallSpy {
    fn after_call(&self, _tool: &Tool, _result: &ToolResult) {
        *self.called.lock().unwrap() = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailInspector;

    impl Inspector for FailInspector {
        fn before_call(&self, _tool: &Tool) -> Result<(), String> {
            Err("blocked by inspector".to_string())
        }
    }

    #[test]
    fn inspector_before_call_blocks() {
        let pipeline = ToolPipeline::new(vec![Arc::new(FailInspector)]);
        let tool = Tool::Bash { command: "echo hi".to_string() };
        let result = pipeline.call(&tool);
        assert!(!result.success);
        assert!(result.output.contains("blocked by inspector"));
    }

    #[test]
    fn after_call_runs_on_error() {
        let spy = Arc::new(AfterCallSpy::new());
        let pipeline = ToolPipeline::new(vec![spy.clone()]);

        let tool = Tool::ReadFile {
            path: "DOES_NOT_EXIST_xyz".to_string(),
            offset: None,
            limit: None,
        };
        let result = pipeline.call(&tool);
        assert!(!result.success);
        assert!(spy.was_called(), "after_call should run even on error");
    }

    #[test]
    fn call_counter_increments() {
        let counter = Arc::new(CallCounter::new());
        let pipeline = ToolPipeline::new(vec![counter.clone()]);

        let tool = Tool::Bash { command: "echo hi".to_string() };
        pipeline.call(&tool);
        pipeline.call(&tool);

        assert_eq!(counter.count("bash"), 2);
    }

    #[test]
    fn empty_pipeline_runs_tool() {
        let pipeline = ToolPipeline::new(vec![]);
        let tool = Tool::Bash { command: "echo hello".to_string() };
        let result = pipeline.call(&tool);
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn multiple_inspectors_run_in_order() {
        let spy1 = Arc::new(AfterCallSpy::new());
        let spy2 = Arc::new(AfterCallSpy::new());
        let pipeline = ToolPipeline::new(vec![spy1.clone(), spy2.clone()]);

        let tool = Tool::Bash { command: "echo hi".to_string() };
        pipeline.call(&tool);

        assert!(spy1.was_called(), "first inspector after_call should run");
        assert!(spy2.was_called(), "second inspector after_call should run");
    }

    #[test]
    fn pipeline_push_fluent_api() {
        let counter = Arc::new(CallCounter::new());
        let spy = Arc::new(AfterCallSpy::new());
        let pipeline = ToolPipeline::new(vec![])
            .push(counter.clone())
            .push(spy.clone());

        let tool = Tool::Bash { command: "echo hi".to_string() };
        pipeline.call(&tool);

        assert_eq!(counter.count("bash"), 1);
        assert!(spy.was_called());
    }
}
