//! Skill system - agent-capable extensions
//!
//! Skills are extensions that the LLM can invoke to perform complex tasks.
//! Unlike plugins, skills are designed to be called by the agent itself
//! through a standardized interface.
//!
//! ## Invocation Flow
//!
//! ```text
//! LLM → "invoke skill: image-gen prompt: sunset over mountains"
//!         ↓
//!    SkillRegistry.find("image-gen")
//!         ↓
//!    SkillInvocation::new(input)
//!         ↓
//!    skill.execute(invocation)
//!         ↓
////!    SkillResult::Success(output) → LLM context
//! ```

use crate::{Plugin, PluginEvent, PluginAction};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// A skill is a plugin that can be invoked by the agent
#[async_trait]
pub trait Skill: Send + Sync {
    /// Unique skill identifier
    fn id(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Description for the agent's skill selection
    fn description(&self) -> &str;

    /// Input schema (JSON Schema) for this skill
    fn input_schema(&self) -> serde_json::Value;

    /// Execute the skill with given input
    async fn execute(&self, invocation: SkillInvocation) -> SkillResult;
}

/// Invocation context for a skill
#[derive(Debug, Clone)]
pub struct SkillInvocation {
    pub skill_id: String,
    pub input: serde_json::Value,
    pub context: SkillContext,
}

impl SkillInvocation {
    pub fn new(skill_id: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            skill_id: skill_id.into(),
            input,
            context: SkillContext::default(),
        }
    }

    pub fn with_context(mut self, ctx: SkillContext) -> Self {
        self.context = ctx;
        self
    }

    /// Parse input as a specific type
    pub fn parse_input<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.input.clone())
    }
}

/// Context passed to skill execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillContext {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub working_directory: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Result of skill execution
#[derive(Debug, Clone)]
pub enum SkillResult {
    Success {
        output: serde_json::Value,
        message: Option<String>,
    },
    Error {
        message: String,
        retryable: bool,
    },
    Partial {
        output: serde_json::Value,
        continuation_token: String,
    },
}

impl SkillResult {
    pub fn success(output: impl Into<serde_json::Value>) -> Self {
        Self::Success {
            output: output.into(),
            message: None,
        }
    }

    pub fn success_with_message(output: impl Into<serde_json::Value>, message: impl Into<String>) -> Self {
        Self::Success {
            output: output.into(),
            message: Some(message.into()),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            retryable: false,
        }
    }

    pub fn retryable_error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            retryable: true,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, SkillResult::Success { .. })
    }
}

/// Skill registry - manages available skills
pub struct SkillRegistry {
    skills: std::sync::RwLock<HashMap<String, Arc<dyn Skill>>>,
    aliases: std::sync::RwLock<HashMap<String, String>>, // alias → skill_id
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: std::sync::RwLock::new(HashMap::new()),
            aliases: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Register a skill
    pub fn register(&self, skill: Arc<dyn Skill>) {
        let id = skill.id().to_string();
        let mut skills = self.skills.write().unwrap();
        skills.insert(id.clone(), skill);
        tracing::info!("Registered skill: {}", id);
    }

    /// Register an alias for a skill
    pub fn register_alias(&self, alias: impl Into<String>, skill_id: impl Into<String>) {
        let mut aliases = self.aliases.write().unwrap();
        aliases.insert(alias.into(), skill_id.into());
    }

    /// Find a skill by ID or alias
    pub fn find(&self, id_or_alias: &str) -> Option<Arc<dyn Skill>> {
        let skills = self.skills.read().unwrap();

        // Try direct lookup
        if let Some(skill) = skills.get(id_or_alias) {
            return Some(skill.clone());
        }

        // Try alias lookup
        let aliases = self.aliases.read().unwrap();
        if let Some(id) = aliases.get(id_or_alias) {
            return skills.get(id).cloned();
        }

        None
    }

    /// List all available skills
    pub fn list(&self) -> Vec<SkillInfo> {
        let skills = self.skills.read().unwrap();
        skills.values()
            .map(|s| SkillInfo {
                id: s.id().to_string(),
                name: s.name().to_string(),
                description: s.description().to_string(),
                input_schema: s.input_schema(),
            })
            .collect()
    }

    /// Invoke a skill by ID or alias
    pub async fn invoke(&self, invocation: SkillInvocation) -> Result<SkillResult, SkillError> {
        let skill = self.find(&invocation.skill_id)
            .ok_or_else(|| SkillError::NotFound(invocation.skill_id.clone()))?;

        Ok(skill.execute(invocation).await)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Skill information for listing
#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Skill errors
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),
    #[error("Skill execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid input: {0}")]
    InvalidInput(#[from] serde_json::Error),
}

// ─────────────────────────────────────────────────────────────────
// Adapter: Plugin → Skill
// ─────────────────────────────────────────────────────────────────

/// Adapter to wrap a Plugin as a Skill
pub struct PluginAsSkill {
    plugin: Arc<dyn Plugin>,
}

impl PluginAsSkill {
    pub fn new(plugin: Arc<dyn Plugin>) -> Self {
        Self { plugin }
    }
}

#[async_trait]
impl Skill for PluginAsSkill {
    fn id(&self) -> &str {
        self.plugin.name()
    }

    fn name(&self) -> &str {
        self.plugin.name()
    }

    fn description(&self) -> &str {
        self.plugin.description().unwrap_or("Plugin as skill")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "The action to perform"
                }
            }
        })
    }

    async fn execute(&self, invocation: SkillInvocation) -> SkillResult {
        let input = invocation.input;
        let event = PluginEvent::MessageReceived {
            role: crate::MessageRole::System,
            content: input.get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("execute")
                .to_string(),
        };

        let actions = self.plugin.on_event(event);

        if actions.is_empty() {
            SkillResult::success(serde_json::json!({ "actions": actions }))
        } else {
            SkillResult::success_with_message(
                serde_json::json!({ "actions": actions }),
                format!("Executed {} actions", actions.len())
            )
        }
    }
}
