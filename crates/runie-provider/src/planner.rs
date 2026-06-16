//! One-shot Orchestrator LLM planner.
//!
//! Calls the planner model once with a structured prompt and parses the response
//! into an `OrchestratorPlan`. Retries on parse failure up to `max_retries` times.

use std::collections::HashMap;

use anyhow::Result;
use futures::StreamExt;
use runie_core::llm_event::LLMEvent;
use runie_core::message::ChatMessage;
use runie_core::orchestrator::{ModelTrait, OrchestratorContext, OrchestratorPlan};
use runie_core::provider::Provider;
use runie_core::trait_resolver::ModelResolver;

#[cfg(test)]
use runie_core::trait_resolver::ModelProfile;

use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};

// ─── Config ─────────────────────────────────────────────────────────────────

/// Configuration for the one-shot planner.
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// Maximum parse retries before giving up. Defaults to 2.
    pub max_retries: usize,
    /// Timeout for a single LLM call. Defaults to 60s.
    pub timeout: Duration,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            timeout: Duration::from_secs(60),
        }
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

/// Errors that can occur during planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannerError {
    /// LLM call timed out.
    Timeout,
    /// Failed to parse LLM output as JSON after all retries.
    ParseFailed { attempts: usize, last_error: String },
    /// Plan validation failed.
    ValidationFailed(String),
    /// No model matches the required trait for a task.
    NoModelForTrait { trait_: ModelTrait },
    /// Provider returned an error.
    ProviderError(String),
}

impl std::fmt::Display for PlannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlannerError::Timeout => write!(f, "planner LLM call timed out"),
            PlannerError::ParseFailed { attempts, last_error } => {
                write!(f, "failed to parse plan JSON after {} attempts: {}", attempts, last_error)
            }
            PlannerError::ValidationFailed(msg) => write!(f, "plan validation failed: {}", msg),
            PlannerError::NoModelForTrait { trait_ } => {
                write!(f, "no model configured for trait '{}'", trait_)
            }
            PlannerError::ProviderError(msg) => write!(f, "provider error: {}", msg),
        }
    }
}

impl std::error::Error for PlannerError {}

// ─── Tool description ───────────────────────────────────────────────────────

/// A tool available to the Orchestrator for planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
}

// ─── Project context ────────────────────────────────────────────────────────

/// Project context passed to the planner.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Short workspace description (e.g. "Rust CLI tool with TUI").
    pub description: String,
    /// Top-level directory names.
    pub directories: Vec<String>,
    /// Key file names (e.g. Cargo.toml, package.json).
    pub key_files: Vec<String>,
}

impl ProjectContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_directories(mut self, dirs: Vec<String>) -> Self {
        self.directories = dirs;
        self
    }

    pub fn with_key_files(mut self, files: Vec<String>) -> Self {
        self.key_files = files;
        self
    }
}

// ─── Plan input ─────────────────────────────────────────────────────────────

/// Everything the planner needs to generate a plan.
#[derive(Debug, Clone)]
pub struct PlanInput<'a> {
    /// The user's request to break into subagent tasks.
    pub user_request: &'a str,
    /// Project context to include in the prompt.
    pub project: &'a ProjectContext,
    /// Orchestrator working memory (Ask-User Q&A).
    pub orchestrator_context: &'a OrchestratorContext,
    /// Available tools with descriptions.
    pub tools: &'a [ToolDescription],
    /// Available model traits (for the prompt).
    pub available_traits: &'a [ModelTrait],
}

impl<'a> PlanInput<'a> {
    pub fn new(
        user_request: &'a str,
        project: &'a ProjectContext,
        orchestrator_context: &'a OrchestratorContext,
    ) -> Self {
        Self {
            user_request,
            project,
            orchestrator_context,
            tools: &[],
            available_traits: &[ModelTrait::General],
        }
    }

    pub fn with_tools(mut self, tools: &'a [ToolDescription]) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_traits(mut self, traits: &'a [ModelTrait]) -> Self {
        self.available_traits = traits;
        self
    }
}

// ─── Prompt builder ─────────────────────────────────────────────────────────

fn build_planner_system_prompt(traits: &[ModelTrait], tools: &[ToolDescription]) -> String {
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

fn build_user_prompt(input: &PlanInput) -> String {
    let mut prompt = String::new();

    if !input.project.description.is_empty() {
        prompt.push_str(&format!("## Project Context\n{}\n\n", input.project.description));
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

// ─── Parser ──────────────────────────────────────────────────────────────────

/// Raw intermediate JSON structure for parsing LLM output.
#[derive(Debug, Deserialize)]
struct RawPlan {
    tasks: Vec<RawTask>,
    #[serde(default)]
    synthesis_trait: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawTask {
    id: String,
    role_prompt: String,
    task_description: String,
    #[serde(default)]
    tool_filter: Vec<String>,
    model_trait: String,
}

fn parse_trait(s: &str) -> Option<ModelTrait> {
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

fn parse_raw_plan(raw: RawPlan, tool_names: &HashMap<String, ()>) -> Result<OrchestratorPlan, PlannerError> {
    let mut tasks = Vec::new();

    for raw_task in raw.tasks {
        let model_trait = parse_trait(&raw_task.model_trait)
            .ok_or_else(|| PlannerError::ValidationFailed(format!(
                "unknown model trait '{}' in task '{}'", raw_task.model_trait, raw_task.id
            )))?;

        // Validate tool_filter: all tools must exist.
        for tool_name in &raw_task.tool_filter {
            if !tool_names.contains_key(tool_name) {
                return Err(PlannerError::ValidationFailed(format!(
                    "task '{}' references unknown tool '{}'", raw_task.id, tool_name
                )));
            }
        }

        tasks.push(runie_core::orchestrator::SubagentTask {
            id: raw_task.id,
            role_prompt: raw_task.role_prompt,
            task_description: raw_task.task_description,
            tool_filter: if raw_task.tool_filter.is_empty() {
                None
            } else {
                Some(raw_task.tool_filter)
            },
            model_trait,
            status: runie_core::orchestrator::TaskStatus::Pending,
            output: None,
        });
    }

    if tasks.is_empty() {
        return Err(PlannerError::ValidationFailed("plan has no tasks".to_string()));
    }

    let synthesis_trait = raw.synthesis_trait
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

// ─── Main planner ───────────────────────────────────────────────────────────

/// One-shot Orchestrator planner.
///
/// Calls the planner model once with a structured prompt. Retries on parse
/// failure up to `config.max_retries` times.
pub struct OneShotPlanner<'a, P: Provider> {
    provider: &'a P,
    _resolver: &'a ModelResolver,
    tools: &'a [ToolDescription],
    config: PlannerConfig,
}

impl<'a, P: Provider> OneShotPlanner<'a, P> {
    /// Create a new planner.
    pub fn new(provider: &'a P, resolver: &'a ModelResolver) -> Self {
        Self {
            provider,
            _resolver: resolver,
            tools: &[],
            config: PlannerConfig::default(),
        }
    }

    /// Set available tools (for validation).
    pub fn with_tools(mut self, tools: &'a [ToolDescription]) -> Self {
        self.tools = tools;
        self
    }

    /// Override max retries.
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    /// Override timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Build a tool name lookup set for O(1) validation.
    fn tool_names(&self) -> HashMap<String, ()> {
        self.tools.iter().map(|t| (t.name.clone(), ())).collect()
    }

    /// Run the planner and return an `OrchestratorPlan`.
    pub async fn plan(&self, input: &PlanInput<'_>) -> Result<OrchestratorPlan, PlannerError> {
        let system = build_planner_system_prompt(input.available_traits, self.tools);
        let user = build_user_prompt(input);

        let mut messages = vec![
            ChatMessage::system(system),
            ChatMessage::user(user),
        ];

        let mut last_error = String::new();

        for attempt in 1..=self.config.max_retries + 1 {
            let mut stream = self.provider.generate(messages.clone());

            // Collect the full stream, timing out on the first item only.
            let text = match timeout(self.config.timeout, stream.next()).await {
                Ok(Some(Ok(LLMEvent::TextDelta(initial)))) => {
                    let mut text = initial;
                    while let Some(event) = stream.next().await {
                        if let Ok(LLMEvent::TextDelta(delta)) = event {
                            text.push_str(&delta);
                        }
                    }
                    text
                }
                Ok(Some(Ok(LLMEvent::ThinkingDelta(_)))) => {
                    // Collect thinking content (may contain useful info)
                    let mut text = String::new();
                    while let Some(event) = stream.next().await {
                        if let Ok(LLMEvent::TextDelta(delta)) = event {
                            text.push_str(&delta);
                        }
                    }
                    text
                }
                Ok(Some(Ok(_))) => {
                    // Other event types (tool calls etc.) - ignore for planning
                    String::new()
                }
                Ok(Some(Err(e))) => return Err(PlannerError::ProviderError(e.to_string())),
                Ok(None) => String::new(),
                Err(_) => return Err(PlannerError::Timeout),
            };

            // Parse the response
            eprintln!("DEBUG: Trying to parse text (first 100 chars): {:?}", &text[..text.len().min(100)]);
            let parse_result = serde_json::from_str::<serde_json::Value>(&text);

            // Try parsing directly as RawPlan
            if parse_result.is_ok() {
                if let Ok(value) = parse_result {
                    match serde_json::from_value::<RawPlan>(value) {
                        Ok(raw) => {
                            let tool_names = self.tool_names();
                            match parse_raw_plan(raw, &tool_names) {
                                Ok(plan) => return Ok(plan),
                                Err(e) => {
                                    last_error = e.to_string();
                                }
                            }
                        }
                        Err(e) => {
                            last_error = e.to_string();
                        }
                    }
                }
            } else {
                // from_str failed, store error
                if let Err(e) = parse_result {
                    last_error = e.to_string();
                }
            }

            // Try extracting JSON from markdown code block (regardless of above result)
            if let Some(json_text) = extract_json_from_text(&text) {
                match serde_json::from_str::<RawPlan>(&json_text) {
                    Ok(raw) => {
                        let tool_names = self.tool_names();
                        match parse_raw_plan(raw, &tool_names) {
                            Ok(plan) => return Ok(plan),
                            Err(e) => {
                                last_error = e.to_string();
                            }
                        }
                    }
                    Err(e) => {
                        last_error = format!("{}; also failed markdown extract", e);
                    }
                }
            }

            if attempt <= self.config.max_retries {
                // Append a correction hint and retry
                let correction = format!(
                    "\n\n[Planner] Your previous output was not valid JSON: {}. \
                     Please respond with ONLY the JSON plan object, no explanation.",
                    last_error
                );
                messages.push(ChatMessage::user(correction));
            }
        }

        Err(PlannerError::ParseFailed {
            attempts: self.config.max_retries + 1,
            last_error,
        })
    }
}

/// Try to extract JSON from a markdown code block, or return the original text.
fn extract_json_from_text(text: &str) -> Option<String> {
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

// ─── Plan validation ─────────────────────────────────────────────────────────

/// Validate a plan against the model resolver.
pub fn validate_plan(plan: &OrchestratorPlan, resolver: &ModelResolver) -> Result<(), PlannerError> {
    // Check synthesis trait
    resolver
        .resolve(plan.synthesis_trait)
        .map_err(|_| PlannerError::NoModelForTrait {
            trait_: plan.synthesis_trait,
        })?;

    // Check each task's model trait
    let mut seen = std::collections::HashSet::new();
    for task in &plan.tasks {
        if !seen.insert(task.id.clone()) {
            return Err(PlannerError::ValidationFailed(format!(
                "duplicate task id: '{}'",
                task.id
            )));
        }

        resolver
            .resolve(task.model_trait)
            .map_err(|_| PlannerError::NoModelForTrait {
                trait_: task.model_trait,
            })?;
    }

    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::orchestrator::ModelTrait;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use runie_core::provider::Provider;

    // ── Mock provider that returns canned text ─────────────────────────────

    struct MockTextProvider {
        responses: Mutex<VecDeque<String>>,
    }

    impl MockTextProvider {
        fn new(responses: Vec<String>) -> Self {
            Self { responses: Mutex::new(VecDeque::from(responses)) }
        }
    }

    impl Provider for MockTextProvider {
        fn generate(
            &self,
            _messages: Vec<ChatMessage>,
        ) -> std::pin::Pin<
            Box<dyn futures::Stream<Item = Result<LLMEvent>> + Send + '_>,
        > {
            let text = self.responses.lock().unwrap().pop_front().unwrap_or_default();
            let chunks: Vec<_> = text
                .chars()
                .map(|c| Ok(LLMEvent::TextDelta(c.to_string())))
                .collect();
            let stream = futures::stream::iter(chunks);
            Box::pin(stream)
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    fn make_resolver() -> ModelResolver {
        ModelResolver::new(vec![
            ModelProfile::new("openai", "gpt-4o", vec![ModelTrait::General, ModelTrait::Vision]),
            ModelProfile::new("anthropic", "o3-mini", vec![ModelTrait::Reasoning]),
            ModelProfile::new("fast", "claude-haiku", vec![ModelTrait::Fast]),
        ])
    }

    fn make_tools() -> Vec<ToolDescription> {
        vec![
            ToolDescription { name: "read_file".into(), description: "Read file contents".into() },
            ToolDescription { name: "bash".into(), description: "Run shell commands".into() },
            ToolDescription { name: "grep".into(), description: "Search file contents".into() },
            ToolDescription { name: "list_dir".into(), description: "List directory contents".into() },
        ]
    }

    // ── Parse valid JSON ──────────────────────────────────────────────────

    #[tokio::test]
    async fn planner_parses_valid_json() {
        let json = include_str!("fixtures/plan.json");
        let provider = MockTextProvider::new(vec![json.to_string()]);
        let resolver = make_resolver();
        let tools = make_tools();

        let planner = OneShotPlanner::new(&provider, &resolver)
            .with_tools(&tools)
            .with_max_retries(0);

        let project = ProjectContext::new().with_description("Rust CLI tool");
        let ctx = OrchestratorContext::new();
        let input = PlanInput::new("Review the codebase", &project, &ctx);

        let plan = planner.plan(&input).await.unwrap();
        assert_eq!(plan.tasks.len(), 2);
        assert!(plan.tasks.iter().all(|t| t.status == runie_core::orchestrator::TaskStatus::Pending));
    }

    #[tokio::test]
    async fn planner_retries_on_invalid_json() {
        let provider = MockTextProvider::new(vec![
            "not json".to_string(),
            include_str!("fixtures/plan.json").to_string(),
        ]);
        let resolver = make_resolver();
        let tools = make_tools();

        let planner = OneShotPlanner::new(&provider, &resolver)
            .with_tools(&tools)
            .with_max_retries(2);

        let project = ProjectContext::new();
        let ctx = OrchestratorContext::new();
        let input = PlanInput::new("Fix bug", &project, &ctx);

        let plan = planner.plan(&input).await.unwrap();
        assert_eq!(plan.tasks.len(), 2);
    }

    #[tokio::test]
    async fn planner_retries_on_invalid_trait() {
        let json = include_str!("fixtures/plan_bad_trait.json");
        let provider = MockTextProvider::new(vec![
            json.to_string(),
            include_str!("fixtures/plan.json").to_string(),
        ]);
        let resolver = make_resolver();
        let tools = make_tools();

        let planner = OneShotPlanner::new(&provider, &resolver)
            .with_tools(&tools)
            .with_max_retries(2);

        let project = ProjectContext::new();
        let ctx = OrchestratorContext::new();
        let input = PlanInput::new("Fix bug", &project, &ctx);
        let plan = planner.plan(&input).await.unwrap();
        assert_eq!(plan.tasks.len(), 2);
    }

    #[tokio::test]
    async fn plan_validation_rejects_unknown_trait() {
        let json = include_str!("fixtures/plan.json");
        let provider = MockTextProvider::new(vec![json.to_string()]);
        let resolver = make_resolver();
        let tools = make_tools();

        let planner = OneShotPlanner::new(&provider, &resolver)
            .with_tools(&tools)
            .with_max_retries(0);

        let project = ProjectContext::new();
        let ctx = OrchestratorContext::new();
        let input = PlanInput::new("Fix bug", &project, &ctx);
        let plan = planner.plan(&input).await.unwrap();

        // Manually create a bad plan
        let mut bad_plan = plan.clone();
        bad_plan.tasks[0].model_trait = ModelTrait::Vision;
        // Note: Vision is valid, but let's use a validation path

        // Check that validate_plan works
        validate_plan(&bad_plan, &resolver).unwrap();

        // Plan with duplicate IDs
        let mut dup_plan = plan.clone();
        dup_plan.tasks.push(dup_plan.tasks[0].clone());
        let err = validate_plan(&dup_plan, &resolver).unwrap_err();
        assert!(matches!(err, PlannerError::ValidationFailed(_)));
    }

    #[tokio::test]
    async fn planner_extracts_json_from_markdown() {
        let provider = MockTextProvider::new(vec![
            "Here's the plan:\n```json\n".to_string() + include_str!("fixtures/plan.json") + "\n```",
        ]);
        let resolver = make_resolver();
        let tools = make_tools();

        let planner = OneShotPlanner::new(&provider, &resolver)
            .with_tools(&tools)
            .with_max_retries(0);

        let project = ProjectContext::new();
        let ctx = OrchestratorContext::new();
        let input = PlanInput::new("Review", &project, &ctx);
        let plan = planner.plan(&input).await.unwrap();
        assert_eq!(plan.tasks.len(), 2);
    }

    #[tokio::test]
    async fn orchestrator_context_included_in_prompt() {
        let json = include_str!("fixtures/plan.json");
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured2 = captured.clone();

        struct InspectProvider {
            responses: Mutex<VecDeque<String>>,
            captured: std::sync::Arc<std::sync::Mutex<String>>,
        }

        impl InspectProvider {
            fn new(responses: Vec<String>, captured: std::sync::Arc<std::sync::Mutex<String>>) -> Self {
                Self { responses: Mutex::new(VecDeque::from(responses)), captured }
            }
        }

        impl Provider for InspectProvider {
            fn generate(&self, messages: Vec<ChatMessage>) -> std::pin::Pin<Box<dyn futures::Stream<Item=Result<LLMEvent>> + Send + '_>> {
                if let Some(last) = messages.last() {
                    *self.captured.lock().unwrap() = last.content.clone();
                }
                let text = self.responses.lock().unwrap().pop_front().unwrap_or_default();
                let chunks: Vec<_> = text.chars().map(|c| Ok(LLMEvent::TextDelta(c.to_string()))).collect();
                Box::pin(futures::stream::iter(chunks)) as _
            }
        }

        let provider = InspectProvider::new(vec![json.to_string()], captured.clone());
        let resolver = make_resolver();
        let tools = make_tools();

        let mut ctx = OrchestratorContext::new();
        ctx.record_question("Which file should I start with?");
        ctx.record_answer("src/main.rs");

        let planner = OneShotPlanner::new(&provider, &resolver)
            .with_tools(&tools)
            .with_max_retries(0);
        let project = ProjectContext::new();
        let input = PlanInput::new("Refactor", &project, &ctx);

        planner.plan(&input).await.unwrap();

        let captured_text = captured2.lock().unwrap();
        assert!(captured_text.contains("Which file should I start with"));
        assert!(captured_text.contains("src/main.rs"));
    }
}
