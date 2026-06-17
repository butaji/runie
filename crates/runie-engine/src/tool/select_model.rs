//! `select_model` tool — resolves a `ModelTrait` to a concrete model.

use std::time::Instant;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{Tool, ToolContext, ToolOutput, ToolStatus};

/// Built-in tool that maps a model trait to a concrete provider/model pair.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectModelTool;

impl SelectModelTool {
    /// Synchronous execute used in tests.
    pub fn execute(&self, input: Value) -> Result<ToolOutput> {
        let start = Instant::now();
        let trait_name = input["trait"]
            .as_str()
            .ok_or_else(|| anyhow!("select_model: missing required field 'trait'"))?;
        let model_trait = parse_trait(trait_name)?;
        let model = resolve_trait(model_trait)?;
        Ok(ToolOutput {
            tool_name: "select_model".to_string(),
            tool_args: input,
            content: model,
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        })
    }
}

#[async_trait]
impl Tool for SelectModelTool {
    fn name(&self) -> &str {
        "select_model"
    }

    fn description(&self) -> &str {
        "Resolve a model trait (fast, general, reasoning, vision, long-context) to a concrete provider/model pair."
    }

    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "trait": {
                    "type": "string",
                    "description": "Model trait to resolve",
                    "enum": ["fast", "general", "reasoning", "vision", "long-context"]
                }
            },
            "required": ["trait"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn requires_approval(&self, _input: &Value) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let start = Instant::now();
        let trait_name = input["trait"]
            .as_str()
            .ok_or_else(|| anyhow!("select_model: missing required field 'trait'"))?
            .to_string();
        let model_trait = parse_trait(&trait_name)?;
        let model = resolve_trait(model_trait)?;
        Ok(ToolOutput {
            tool_name: "select_model".to_string(),
            tool_args: input,
            content: model,
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Success,
        })
    }
}

fn parse_trait(name: &str) -> Result<runie_core::orchestrator::ModelTrait> {
    use runie_core::orchestrator::ModelTrait;
    match name.to_lowercase().as_str() {
        "fast" => Ok(ModelTrait::Fast),
        "general" => Ok(ModelTrait::General),
        "reasoning" => Ok(ModelTrait::Reasoning),
        "vision" => Ok(ModelTrait::Vision),
        "long-context" => Ok(ModelTrait::LongContext),
        _ => Err(anyhow!("unknown model trait: {name}")),
    }
}

fn resolve_trait(trait_: runie_core::orchestrator::ModelTrait) -> Result<String> {
    let catalog = runie_core::model_catalog::model_catalog();
    let preferred = preferred_full(trait_);
    if let Some(m) = catalog.iter().find(|m| m.full() == preferred) {
        return Ok(m.full());
    }
    let fallback = fallback_full(trait_, &catalog);
    fallback.ok_or_else(|| anyhow!("no model available for trait {trait_}"))
}

fn preferred_full(trait_: runie_core::orchestrator::ModelTrait) -> &'static str {
    use runie_core::orchestrator::ModelTrait;
    match trait_ {
        ModelTrait::Fast => "openai/gpt-4o-mini",
        ModelTrait::General => "openai/gpt-4o",
        ModelTrait::Reasoning => "openai/o1",
        ModelTrait::Vision => "openai/gpt-4o",
        ModelTrait::LongContext => "google/gemini-2.5-pro",
    }
}

fn fallback_full(
    trait_: runie_core::orchestrator::ModelTrait,
    catalog: &[runie_core::model_catalog::ModelInfo],
) -> Option<String> {
    use runie_core::orchestrator::ModelTrait;
    match trait_ {
        ModelTrait::Fast => catalog
            .iter()
            .find(|m| {
                m.name.contains("mini") || m.name.contains("flash") || m.name.contains("haiku")
            })
            .map(|m| m.full()),
        ModelTrait::Reasoning => catalog
            .iter()
            .find(|m| {
                m.capabilities.supports_reasoning
                    || m.name.contains("reasoner")
                    || m.name.contains("o1")
            })
            .map(|m| m.full()),
        ModelTrait::Vision => catalog
            .iter()
            .find(|m| m.capabilities.supports_vision)
            .map(|m| m.full()),
        ModelTrait::LongContext => catalog
            .iter()
            .max_by_key(|m| m.capabilities.max_context_tokens)
            .map(|m| m.full()),
        ModelTrait::General => catalog.first().map(|m| m.full()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_model_resolves_general() {
        let tool = SelectModelTool;
        let result = tool
            .execute(serde_json::json!({"trait": "general"}))
            .unwrap();
        assert!(result.content.contains('/'));
    }

    #[test]
    fn select_model_rejects_unknown_trait() {
        let tool = SelectModelTool;
        assert!(tool
            .execute(serde_json::json!({"trait": "unknown"}))
            .is_err());
    }
}
