use std::collections::HashMap;

use runie_core::orchestrator::{
    OrchestratorPlan, SubagentTask, SynthesisConfig, TaskStatus,
};
use runie_core::trait_resolver::ModelTrait;
use serde::Deserialize;

use crate::planner::error::PlannerError;

/// Raw intermediate JSON structure for parsing LLM output.
#[derive(Debug, Deserialize)]
pub(crate) struct RawPlan {
    tasks: Vec<RawTask>,
    #[serde(default)]
    synthesis_trait: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawTask {
    id: String,
    role_prompt: String,
    task_description: String,
    #[serde(default)]
    tool_filter: Vec<String>,
    model_trait: String,
}

pub(crate) fn parse_trait(s: &str) -> Option<ModelTrait> {
    match s.to_lowercase().as_str() {
        "general" => Some(ModelTrait::General),
        "reasoning" => Some(ModelTrait::Reasoning),
        "fast" => Some(ModelTrait::Fast),
        "vision" => Some(ModelTrait::Vision),
        "long-context" | "longcontext" => Some(ModelTrait::LongContext),
        "code" => Some(ModelTrait::Fast), // code maps to Fast
        _ => None,
    }
}

pub(crate) fn parse_raw_plan(
    raw: RawPlan,
    tool_names: &HashMap<String, ()>,
) -> Result<OrchestratorPlan, PlannerError> {
    let tasks: Result<Vec<_>, _> = raw
        .tasks
        .into_iter()
        .map(|raw_task| parse_task(raw_task, tool_names))
        .collect();
    let tasks = tasks?;

    ensure_has_tasks(&tasks)?;

    let synthesis_trait = raw
        .synthesis_trait
        .as_ref()
        .and_then(|s| parse_trait(s))
        .unwrap_or(ModelTrait::General);

    Ok(OrchestratorPlan {
        tasks,
        synthesis_trait,
        summary: raw.summary,
        rationale: raw.rationale,
        synthesis: SynthesisConfig::default(),
    })
}

fn parse_task(
    raw: RawTask,
    tool_names: &HashMap<String, ()>,
) -> Result<SubagentTask, PlannerError> {
    let model_trait = parse_trait(&raw.model_trait).ok_or_else(|| {
        PlannerError::ValidationFailed(format!(
            "unknown model trait '{}' in task '{}'",
            raw.model_trait, raw.id
        ))
    })?;

    validate_tool_filter(tool_names, &raw.tool_filter, &raw.id)?;

    Ok(SubagentTask {
        id: raw.id,
        role_prompt: raw.role_prompt,
        task_description: raw.task_description,
        tool_filter: optional_tool_filter(raw.tool_filter),
        model_trait,
        status: TaskStatus::Pending,
    })
}

fn validate_tool_filter(
    tool_names: &HashMap<String, ()>,
    filter: &[String],
    task_id: &str,
) -> Result<(), PlannerError> {
    for tool_name in filter {
        if !tool_names.contains_key(tool_name) {
            return Err(PlannerError::ValidationFailed(format!(
                "task '{}' references unknown tool '{}'",
                task_id, tool_name
            )));
        }
    }
    Ok(())
}

fn optional_tool_filter(filter: Vec<String>) -> Option<Vec<String>> {
    if filter.is_empty() {
        None
    } else {
        Some(filter)
    }
}

fn ensure_has_tasks(tasks: &[SubagentTask]) -> Result<(), PlannerError> {
    if tasks.is_empty() {
        Err(PlannerError::ValidationFailed(
            "plan has no tasks".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Try to extract JSON from a markdown code block, or return the original text.
pub(crate) fn extract_json_from_text(text: &str) -> Option<String> {
    let text = text.trim();
    // Find first ```json ... ``` or ``` ... ```
    if let Some(start) = text.find("```json") {
        let after_start = text[start + 7..].trim_start();
        if let Some(end) = after_start.find("```") {
            return Some(after_start[..end].trim().to_string());
        }
    }
    if let Some(start) = text.find("```") {
        let after_start = text[start + 3..].trim_start();
        if let Some(end) = after_start.find("```") {
            return Some(after_start[..end].trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod extract_tests {
    use super::*;

    #[test]
    fn extracts_json_from_markdown_with_json_lang() {
        let input = "Here's the plan:\n```json\n{\"tasks\": []}\n```\n";
        let result = extract_json_from_text(input);
        assert!(result.is_some());
        let extracted = result.unwrap();
        assert!(extracted.starts_with("{"));
        assert!(serde_json::from_str::<serde_json::Value>(&extracted).is_ok());
    }
}
