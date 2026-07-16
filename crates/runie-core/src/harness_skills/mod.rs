//! Harness Skill Framework — Event-bus interceptors that wrap the agent turn.
//!
//! Skills are default-on, configurable, and togglable harness behaviors.
//! They register hooks: `on_turn_start`, `on_tool_call`, `on_turn_end`.
//!
//! See `docs/Architecture.md#harness-skills` for the high-level design.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod hashline_edit;
mod loop_detector;
mod startup_context;
mod tool_schema_enricher;
mod verification_loop;

pub use hashline_edit::{HashlineEdit, HashlineEditConfig, HashlineEditSkill};
pub use loop_detector::{LoopDetectorConfig, LoopDetectorSkill};
pub use startup_context::{StartupContextConfig, StartupContextSkill};
pub use tool_schema_enricher::{ToolSchemaEnricherConfig, ToolSchemaEnricherSkill};
pub use verification_loop::{VerificationConfig, VerificationLoopSkill};

/// A harness skill that intercepts agent turn lifecycle events.
#[async_trait]
pub trait HarnessSkill: Send + Sync {
    /// Human-readable name for diagnostics.
    fn name(&self) -> &str;

    /// Called before the LLM call.
    fn on_turn_start(&self, _ctx: &TurnStartCtx) -> TurnStartResult {
        TurnStartResult::Continue
    }

    /// Called before and after each tool execution.
    fn on_tool_call(&self, _ctx: &ToolCallCtx) -> ToolCallResult {
        ToolCallResult::Continue
    }

    /// Called after the model declares completion.
    async fn on_turn_end(&self, _ctx: &TurnEndCtx) -> TurnEndResult {
        TurnEndResult::Continue
    }
}

// ---------------------------------------------------------------------------
// Hook input/output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TurnStartCtx {
    /// The user's message content.
    pub message: String,
    /// System prompt being used.
    pub system_prompt: String,
    /// Configured skills context.
    pub skills_context: String,
}

#[derive(Debug, Clone, Default)]
pub enum TurnStartResult {
    /// Continue with the turn as normal.
    #[default]
    Continue,
    /// Skip the LLM call, use this message instead.
    SkipWithMessage(String),
    /// Abort the turn with an error message.
    Abort(String),
}

#[derive(Debug, Clone)]
pub struct ToolCallCtx {
    /// Tool name (e.g., "bash", "read_file").
    pub tool_name: String,
    /// Tool input arguments as JSON.
    pub tool_input: serde_json::Value,
    /// Phase: before or after execution.
    pub phase: ToolCallPhase,
    /// Tool output (available in `After` phase).
    pub tool_output: Option<String>,
    /// Whether the tool call succeeded.
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallPhase {
    Before,
    After,
}

#[derive(Debug, Clone, Default)]
pub enum ToolCallResult {
    /// Continue with tool execution (or continue after).
    #[default]
    Continue,
    /// Skip this tool call, return mock output.
    SkipWithOutput(String),
    /// Abort with error.
    Abort(String),
}

#[derive(Debug, Clone)]
pub struct TurnEndCtx {
    /// The final assistant message.
    pub assistant_message: String,
    /// Number of tool calls made.
    pub tool_call_count: usize,
    /// Whether the turn completed successfully.
    pub success: bool,
}

#[derive(Debug, Clone, Default)]
pub enum TurnEndResult {
    /// Turn is complete.
    #[default]
    Continue,
    /// Request another LLM call (e.g., verification loop).
    RequestAnotherPass,
    /// Abort with error.
    Abort(String),
}

// ---------------------------------------------------------------------------
// Skill configuration
// ---------------------------------------------------------------------------

/// Configuration for a single harness skill.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SkillConfig {
    /// Whether the skill is enabled. Defaults to true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Skill-specific configuration. Free-form for flexibility.
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

/// Configuration for the entire harness section.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HarnessConfig {
    /// Global harness settings.
    #[serde(default)]
    pub skills: HashMap<String, SkillConfig>,
}

// ---------------------------------------------------------------------------
// Skill registry
// ---------------------------------------------------------------------------

/// Registry that manages harness skills and dispatches hooks.
#[derive(Default)]
pub struct SkillRegistry {
    skills: Vec<Box<dyn HarnessSkill>>,
    config: HarnessConfig,
}

impl SkillRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
            config: HarnessConfig::default(),
        }
    }

    /// Register a skill.
    pub fn register(&mut self, skill: impl HarnessSkill + 'static) {
        self.skills.push(Box::new(skill));
    }

    /// Update configuration for skills.
    pub fn set_config(&mut self, config: HarnessConfig) {
        self.config = config;
    }

    /// Get the current configuration.
    pub fn config(&self) -> &HarnessConfig {
        &self.config
    }

    /// Get names of enabled skills.
    pub fn enabled_skills(&self) -> Vec<&str> {
        self.skills
            .iter()
            .filter(|s| self.is_enabled(s.name()))
            .map(|s| s.name())
            .collect()
    }

    /// Check if a skill is enabled in config.
    fn is_enabled(&self, name: &str) -> bool {
        self.config
            .skills
            .get(name)
            .map(|c| c.enabled)
            .unwrap_or(true) // Default to enabled
    }

    /// Dispatch `on_turn_start` to all enabled skills.
    pub fn on_turn_start(&self, ctx: &TurnStartCtx) -> TurnStartResult {
        let mut result = TurnStartResult::Continue;
        for skill in &self.skills {
            if !self.is_enabled(skill.name()) {
                continue;
            }
            let r = skill.on_turn_start(ctx);
            match &r {
                TurnStartResult::Continue => {}
                TurnStartResult::SkipWithMessage(_) => {
                    result = r;
                    break;
                }
                TurnStartResult::Abort(_) => {
                    result = r;
                    break;
                }
            }
        }
        result
    }

    /// Dispatch `on_tool_call` to all enabled skills.
    pub fn on_tool_call(&self, ctx: &ToolCallCtx) -> ToolCallResult {
        let mut result = ToolCallResult::Continue;
        for skill in &self.skills {
            if !self.is_enabled(skill.name()) {
                continue;
            }
            let r = skill.on_tool_call(ctx);
            match &r {
                ToolCallResult::Continue => {}
                ToolCallResult::SkipWithOutput(_) => {
                    result = r;
                    break;
                }
                ToolCallResult::Abort(_) => {
                    result = r;
                    break;
                }
            }
        }
        result
    }

    /// Dispatch `on_turn_end` to all enabled skills.
    pub async fn on_turn_end(&self, ctx: &TurnEndCtx) -> TurnEndResult {
        let mut result = TurnEndResult::Continue;
        for skill in &self.skills {
            if !self.is_enabled(skill.name()) {
                continue;
            }
            let r = skill.on_turn_end(ctx).await;
            match &r {
                TurnEndResult::Continue => {}
                TurnEndResult::RequestAnotherPass => {
                    result = r;
                    break;
                }
                TurnEndResult::Abort(_) => {
                    result = r;
                    break;
                }
            }
        }
        result
    }
}
