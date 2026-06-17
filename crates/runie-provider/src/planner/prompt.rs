use crate::planner::types::{PlanInput, ProjectContext, ToolDescription};
use runie_core::trait_resolver::ModelTrait;

const PLANNER_SYSTEM_TEMPLATE: &str = r#"You are the Runie Orchestrator planner. Your job is to decompose a user's
request into a set of isolated subagent tasks that can be executed in parallel.

## Available Model Traits

{trait_list}

Pick the most appropriate trait for each task based on its nature:
- Code tasks → `code` or `fast`
- Planning / analysis → `reasoning`
- Image/screenshot tasks → `vision`
- Large codebase tasks → `long-context`
- Everything else → `general`

## Available Tools

{tool_list}

## Output Format

Respond ONLY with a valid JSON object conforming to this schema:

{{
  "tasks": [
    {{
      "id": "unique-task-id",
      "role_prompt": "Detailed prompt for this subagent role (1-3 sentences)",
      "task_description": "Specific description of what this task should accomplish",
      "tool_filter": ["list", "of", "allowed", "tool", "names"],
      "model_trait": "one of the available traits above"
    }}
  ],
  "synthesis_trait": "trait for the final synthesis step",
  "summary": "One-sentence summary of the overall plan",
  "rationale": "Brief explanation of why the plan is structured this way"
}}

## Rules

1. Output ONLY the JSON. No markdown, no explanation, no preamble.
2. Each task must have a unique `id`.
3. `tool_filter` is optional; omit or use empty array if no restrictions.
4. The plan must be feasible: every referenced tool must exist.
5. If the user request is ambiguous, ask ONE follow-up question using the
   `ask_user` tool call in a task's description (e.g. "Use ask_user: {{'question': '...'}}").
"#;

pub(crate) fn build_planner_system_prompt(
    traits: &[ModelTrait],
    tools: &[ToolDescription],
) -> String {
    let trait_list = format_trait_list(traits);
    let tool_list = format_tool_list(tools);
    PLANNER_SYSTEM_TEMPLATE
        .replace("{trait_list}", &trait_list)
        .replace("{tool_list}", &tool_list)
}

fn format_trait_list(traits: &[ModelTrait]) -> String {
    traits
        .iter()
        .map(|t| format!("- {}: {}", t, t.label()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_tool_list(tools: &[ToolDescription]) -> String {
    if tools.is_empty() {
        return "No tools available.".to_string();
    }
    tools
        .iter()
        .map(|t| format!("- {}: {}", t.name, t.description))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn build_user_prompt(input: &PlanInput) -> String {
    let mut prompt = String::new();

    push_project_context(&mut prompt, input.project);
    push_directory_context(&mut prompt, input.project);
    push_key_files(&mut prompt, input.project);
    push_clarifying_answers(&mut prompt, input.orchestrator_context);

    prompt.push_str(&format!("## User Request\n{}\n", input.user_request));
    prompt
}

fn push_project_context(prompt: &mut String, project: &ProjectContext) {
    if !project.description.is_empty() {
        prompt.push_str(&format!(
            "## Project Context\n{}\n\n",
            project.description
        ));
    }
}

fn push_directory_context(prompt: &mut String, project: &ProjectContext) {
    if !project.directories.is_empty() {
        prompt.push_str(&format!(
            "## Directory Structure\n{}\n\n",
            project.directories.join("/")
        ));
    }
}

fn push_key_files(prompt: &mut String, project: &ProjectContext) {
    if !project.key_files.is_empty() {
        prompt.push_str(&format!(
            "## Key Files\n{}\n\n",
            project.key_files.join(", ")
        ));
    }
}

fn push_clarifying_answers(
    prompt: &mut String,
    context: &runie_core::orchestrator::OrchestratorContext,
) {
    if !context.is_empty() {
        prompt.push_str("## Clarifying Answers\n");
        for entry in context.dialogue() {
            match entry {
                runie_core::orchestrator::DialogueEntry::Question(q) => {
                    prompt.push_str(&format!("Q: {}\n", q));
                }
                runie_core::orchestrator::DialogueEntry::Answer(a) => {
                    prompt.push_str(&format!("A: {}\n", a));
                }
            }
        }
        prompt.push('\n');
    }
}
