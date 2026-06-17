use std::collections::HashMap;

use runie_core::orchestrator::{OrchestratorPlan, SubagentTask, TaskStatus};
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
    let mut tasks = Vec::new();

    for raw_task in raw.tasks {
        let model_trait = parse_trait(&raw_task.model_trait).ok_or_else(|| {
            PlannerError::ValidationFailed(format!(
                "unknown model trait '{}' in task '{}'",
                raw_task.model_trait, raw_task.id
            ))
        })?;

        // Validate tool_filter: all tools must exist.
        for tool_name in &raw_task.tool_filter {
            if !tool_names.contains_key(tool_name) {
                return Err(PlannerError::ValidationFailed(format!(
                    "task '{}' references unknown tool '{}'",
                    raw_task.id, tool_name
                )));
            }
        }

        tasks.push(SubagentTask {
            id: raw_task.id,
            role_prompt: raw_task.role_prompt,
            task_description: raw_task.task_description,
            tool_filter: if raw_task.tool_filter.is_empty() {
                None
            } else {
                Some(raw_task.tool_filter)
            },
            model_trait,
            status: TaskStatus::Pending,
        });
    }

    if tasks.is_empty() {
        return Err(PlannerError::ValidationFailed(
            "plan has no tasks".to_string(),
        ));
    }

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
    })
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
