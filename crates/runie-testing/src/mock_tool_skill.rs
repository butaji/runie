//! Shared mock tool skill for replay tests.
//!
//! Provides a `HarnessSkill` that returns canned output for configured tool names,
//! letting agent-turn tests run without real IO. Supports builder API for error
//! simulation and call-order verification.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use runie_core::harness_skills::{
    HarnessSkill, SkillRegistry, ToolCallCtx, ToolCallPhase, ToolCallResult,
};

/// Result type for mock tool execution.
#[derive(Debug, Clone)]
pub enum ToolResult {
    /// Tool executed successfully with output.
    Success(String),
    /// Tool execution failed with error message.
    Error(String),
}

impl ToolResult {
    /// Convert to the harness skill `ToolCallResult`.
    fn to_call_result(self) -> ToolCallResult {
        match self {
            ToolResult::Success(output) => ToolCallResult::SkipWithOutput(output),
            ToolResult::Error(msg) => ToolCallResult::Abort(msg),
        }
    }
}

/// Builder for `MockToolSkill` with support for error simulation and call-order verification.
#[derive(Default)]
pub struct MockToolSkillBuilder {
    /// Map of tool name -> result.
    results: HashMap<String, ToolResult>,
    /// Expected call sequence (empty = no order verification).
    expected_calls: Vec<String>,
}

impl MockToolSkillBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a result for a tool name.
    ///
    /// # Examples
    /// ```
    /// use runie_testing::mock_tool_skill::{MockToolSkill, ToolResult};
    /// let skill = MockToolSkill::builder()
    ///     .when("bash", ToolResult::Success("hello".to_string()))
    ///     .when("read_file", ToolResult::Error("not found".to_string()))
    ///     .build();
    /// ```
    pub fn when(mut self, name: &str, result: ToolResult) -> Self {
        self.results.insert(name.to_string(), result);
        self
    }

    /// Set the expected call order. Verification happens lazily on `build()`.
    ///
    /// # Examples
    /// ```
    /// use runie_testing::mock_tool_skill::{MockToolSkill, ToolResult};
    /// let skill = MockToolSkill::builder()
    ///     .when("list_dir", ToolResult::Success(".".to_string()))
    ///     .when("read_file", ToolResult::Success("content".to_string()))
    ///     .expect_calls(vec!["list_dir", "read_file"])
    ///     .build();
    /// ```
    pub fn expect_calls(mut self, calls: Vec<&str>) -> Self {
        self.expected_calls = calls.into_iter().map(String::from).collect();
        self
    }

    /// Build the `MockToolSkill` with the configured results and call expectations.
    pub fn build(self) -> MockToolSkill {
        MockToolSkill {
            results: self.results,
            expected_calls: Arc::new(Mutex::new(Vec::new())),
            expected_sequence: self.expected_calls,
        }
    }
}

/// A harness skill that short-circuits tool execution with canned output.
pub struct MockToolSkill {
    /// Map of tool name -> result.
    results: HashMap<String, ToolResult>,
    /// Recorded call sequence (for verification).
    expected_calls: Arc<Mutex<Vec<String>>>,
    /// Expected call order (empty = no verification).
    expected_sequence: Vec<String>,
}

impl MockToolSkill {
    /// Build a skill that returns `output` for every call to tool `name`.
    ///
    /// This is the simple constructor for backward compatibility.
    pub fn new(outputs: HashMap<String, String>) -> Self {
        let results = outputs
            .into_iter()
            .map(|(k, v)| (k, ToolResult::Success(v)))
            .collect();
        Self {
            results,
            expected_calls: Arc::new(Mutex::new(Vec::new())),
            expected_sequence: Vec::new(),
        }
    }

    /// Build a skill using the builder API.
    ///
    /// # Examples
    /// ```
    /// use runie_testing::mock_tool_skill::{MockToolSkill, ToolResult};
    /// let skill = MockToolSkill::builder()
    ///     .when("bash", ToolResult::Success("hello".to_string()))
    ///     .when("read_file", ToolResult::Error("not found".to_string()))
    ///     .build();
    /// ```
    pub fn builder() -> MockToolSkillBuilder {
        MockToolSkillBuilder::new()
    }

    /// Verify that expected calls match actual calls.
    /// Call this after the test to assert call order.
    pub fn verify_calls(&self) -> Result<(), String> {
        if self.expected_sequence.is_empty() {
            return Ok(());
        }
        let actual = self.expected_calls.lock();
        if *actual != self.expected_sequence {
            return Err(format!(
                "Expected calls {:?}, got {:?}",
                self.expected_sequence, *actual
            ));
        }
        Ok(())
    }

    /// Get the recorded call sequence.
    pub fn recorded_calls(&self) -> Vec<String> {
        self.expected_calls.lock().clone()
    }
}

#[async_trait]
impl HarnessSkill for MockToolSkill {
    fn name(&self) -> &str {
        "mock_tools"
    }

    fn on_tool_call(&self, ctx: &ToolCallCtx) -> ToolCallResult {
        if ctx.phase != ToolCallPhase::Before {
            return ToolCallResult::Continue;
        }
        // Record the call
        self.expected_calls.lock().push(ctx.tool_name.clone());
        // Return configured result or Continue for unknown tools
        self.results
            .get(&ctx.tool_name)
            .cloned()
            .map(ToolResult::to_call_result)
            .unwrap_or(ToolCallResult::Continue)
    }
}

/// Build a `SkillRegistry` with canned outputs for `list_dir` and `bash`.
pub fn mock_tool_skill() -> SkillRegistry {
    let outputs = HashMap::from([
        ("list_dir".to_string(), "Cargo.toml\nREADME.md\n".to_string()),
        ("bash".to_string(), "hello\n".to_string()),
    ]);
    let mut registry = SkillRegistry::new();
    registry.register(MockToolSkill::new(outputs));
    registry
}

/// Build a `SkillRegistry` with canned outputs for `list_dir` and `read_file`
/// (used by MiniMax replay tests).
pub fn mock_tool_skill_minimax() -> SkillRegistry {
    let outputs = HashMap::from([
        (
            "list_dir".to_string(),
            "Cargo.toml (file)\nREADME.md (file)\n".to_string(),
        ),
        (
            "read_file".to_string(),
            "# Runie\n\nA terminal AI assistant.".to_string(),
        ),
    ]);
    let mut registry = SkillRegistry::new();
    registry.register(MockToolSkill::new(outputs));
    registry
}

/// A skill that records the last `ToolCallCtx` it received.
///
/// The shared `ctx` Arc is exposed so callers can `take()` the value after the
/// skill has been moved into a `SkillRegistry`.
pub struct RecordingSkill {
    /// Shared handle to the recorded context — accessible after the skill is
    /// moved into a registry.
    pub ctx: Arc<Mutex<Option<ToolCallCtx>>>,
}

impl RecordingSkill {
    /// Build a recording skill.
    pub fn new() -> Self {
        Self {
            ctx: Arc::new(Mutex::new(None)),
        }
    }

    /// Take and return the last recorded context.
    pub fn take(&self) -> Option<ToolCallCtx> {
        self.ctx.lock().take()
    }
}

impl Default for RecordingSkill {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessSkill for RecordingSkill {
    fn name(&self) -> &str {
        "recording"
    }

    fn on_tool_call(&self, ctx: &ToolCallCtx) -> ToolCallResult {
        *self.ctx.lock() = Some(ctx.clone());
        ToolCallResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_tool_skill_skips_with_output() {
        let skill = MockToolSkill::new([("list_dir".into(), "Cargo.toml\n".into())].into());
        let result = skill.on_tool_call(&ToolCallCtx {
            tool_name: "list_dir".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        });
        assert!(matches!(result, ToolCallResult::SkipWithOutput(_)));
    }

    #[test]
    fn mock_tool_skill_continues_for_unknown_tool() {
        let skill = MockToolSkill::new([("list_dir".into(), "Cargo.toml\n".into())].into());
        let result = skill.on_tool_call(&ToolCallCtx {
            tool_name: "bash".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        });
        assert!(matches!(result, ToolCallResult::Continue));
    }

    #[test]
    fn recording_skill_stores_ctx() {
        let skill = RecordingSkill::new();
        let input = serde_json::json!({"path": "src/main.rs"});
        skill.on_tool_call(&ToolCallCtx {
            tool_name: "read_file".into(),
            tool_input: input.clone(),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        });
        let ctx = skill.take().unwrap();
        assert_eq!(ctx.tool_input, input);
    }

    #[test]
    fn builder_when_success() {
        let skill = MockToolSkill::builder()
            .when("bash", ToolResult::Success("hello".to_string()))
            .build();

        let result = skill.on_tool_call(&ToolCallCtx {
            tool_name: "bash".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        });
        match result {
            ToolCallResult::SkipWithOutput(s) => assert_eq!(s, "hello"),
            _ => panic!("Expected SkipWithOutput"),
        }
    }

    #[test]
    fn builder_when_error() {
        let skill = MockToolSkill::builder()
            .when("read_file", ToolResult::Error("not found".to_string()))
            .build();

        let result = skill.on_tool_call(&ToolCallCtx {
            tool_name: "read_file".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        });
        match result {
            ToolCallResult::Abort(s) => assert_eq!(s, "not found"),
            _ => panic!("Expected Abort"),
        }
    }

    #[test]
    fn builder_expect_calls_success() {
        let skill = MockToolSkill::builder()
            .when("list_dir", ToolResult::Success(".".to_string()))
            .when("read_file", ToolResult::Success("content".to_string()))
            .expect_calls(vec!["list_dir", "read_file"])
            .build();

        // Simulate calls in expected order
        skill.on_tool_call(&ToolCallCtx {
            tool_name: "list_dir".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        });
        skill.on_tool_call(&ToolCallCtx {
            tool_name: "read_file".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        });

        assert!(skill.verify_calls().is_ok());
        assert_eq!(skill.recorded_calls(), vec!["list_dir", "read_file"]);
    }

    #[test]
    fn builder_expect_calls_failure() {
        let skill = MockToolSkill::builder()
            .when("list_dir", ToolResult::Success(".".to_string()))
            .when("read_file", ToolResult::Success("content".to_string()))
            .expect_calls(vec!["list_dir", "read_file"])
            .build();

        // Simulate calls in wrong order
        skill.on_tool_call(&ToolCallCtx {
            tool_name: "read_file".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        });
        skill.on_tool_call(&ToolCallCtx {
            tool_name: "list_dir".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        });

        assert!(skill.verify_calls().is_err());
    }

    #[test]
    fn builder_chaining() {
        let skill = MockToolSkill::builder()
            .when("bash", ToolResult::Success("hello".to_string()))
            .when("read_file", ToolResult::Error("not found".to_string()))
            .when("list_dir", ToolResult::Success(".".to_string()))
            .build();

        // Verify all results
        let ctx = ToolCallCtx {
            tool_name: "bash".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        };
        assert!(matches!(
            skill.on_tool_call(&ctx),
            ToolCallResult::SkipWithOutput(s) if s == "hello"
        ));

        let ctx = ToolCallCtx {
            tool_name: "read_file".into(),
            tool_input: serde_json::json!({}),
            phase: ToolCallPhase::Before,
            tool_output: None,
            success: None,
        };
        assert!(matches!(
            skill.on_tool_call(&ctx),
            ToolCallResult::Abort(s) if s == "not found"
        ));
    }
}
