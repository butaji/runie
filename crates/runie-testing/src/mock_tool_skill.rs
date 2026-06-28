//! Shared mock tool skill for replay tests.
//!
//! Provides a `HarnessSkill` that returns canned output for configured tool names,
//! letting agent-turn tests run without real IO.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use runie_core::harness_skills::{
    HarnessSkill, SkillRegistry, ToolCallCtx, ToolCallPhase, ToolCallResult,
};

/// A harness skill that short-circuits tool execution with canned output.
pub struct MockToolSkill {
    outputs: HashMap<String, String>,
}

impl MockToolSkill {
    /// Build a skill that returns `output` for every call to tool `name`.
    pub fn new(outputs: HashMap<String, String>) -> Self {
        Self { outputs }
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
        self.outputs
            .get(&ctx.tool_name)
            .cloned()
            .map(ToolCallResult::SkipWithOutput)
            .unwrap_or(ToolCallResult::Continue)
    }
}

/// Build a `SkillRegistry` with canned outputs for `list_dir` and `bash`.
pub fn mock_tool_skill() -> SkillRegistry {
    let mut outputs = HashMap::new();
    outputs.insert("list_dir".to_string(), "Cargo.toml\nREADME.md\n".to_string());
    outputs.insert("bash".to_string(), "hello\n".to_string());
    let mut registry = SkillRegistry::new();
    registry.register(MockToolSkill::new(outputs));
    registry
}

/// Build a `SkillRegistry` with canned outputs for `list_dir` and `read_file`
/// (used by MiniMax replay tests).
pub fn mock_tool_skill_minimax() -> SkillRegistry {
    let mut outputs = HashMap::new();
    outputs.insert(
        "list_dir".to_string(),
        "Cargo.toml (file)\nREADME.md (file)\n".to_string(),
    );
    outputs.insert(
        "read_file".to_string(),
        "# Runie\n\nA terminal AI assistant.".to_string(),
    );
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
        Self { ctx: Arc::new(Mutex::new(None)) }
    }

    /// Take and return the last recorded context.
    pub fn take(&self) -> Option<ToolCallCtx> {
        self.ctx.lock().unwrap().take()
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
        *self.ctx.lock().unwrap() = Some(ctx.clone());
        ToolCallResult::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_tool_skill_skips_with_output() {
        let skill = MockToolSkill::new(
            [("list_dir".into(), "Cargo.toml\n".into())].into(),
        );
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
        let skill = MockToolSkill::new(
            [("list_dir".into(), "Cargo.toml\n".into())].into(),
        );
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
}
