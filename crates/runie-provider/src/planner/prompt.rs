use crate::planner::types::{PlanInput, ToolDescription};
use runie_core::trait_resolver::ModelTrait;

pub(crate) fn build_planner_system_prompt(
    traits: &[ModelTrait],
    tools: &[ToolDescription],
) -> String {
    let trait_list = traits
        .iter()
        .map(|t| format!("- {}: {}", t, t.label()))
        .collect::<Vec<_>>()
        .join("\n");

    let tool_list = if tools.is_empty() {
        "No tools available.".to_string()
    } else {
        tools
            .iter()
            .map(|t| format!("- {}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"You are the Runie Orchestrator planner. Your job is to decompose a user's
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
"#
    )
}

pub(crate) fn build_user_prompt(input: &PlanInput) -> String {
    let mut prompt = String::new();

    if !input.project.description.is_empty() {
        prompt.push_str(&format!(
            "## Project Context\n{}\n\n",
            input.project.description
        ));
    }

    if !input.project.directories.is_empty() {
        prompt.push_str(&format!(
            "## Directory Structure\n{}\n\n",
            input.project.directories.join("/")
        ));
    }

    if !input.project.key_files.is_empty() {
        prompt.push_str(&format!(
            "## Key Files\n{}\n\n",
            input.project.key_files.join(", ")
        ));
    }

    if !input.orchestrator_context.is_empty() {
        prompt.push_str("## Clarifying Answers\n");
        for entry in input.orchestrator_context.dialogue() {
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

    prompt.push_str(&format!("## User Request\n{}\n", input.user_request));
    prompt
}
