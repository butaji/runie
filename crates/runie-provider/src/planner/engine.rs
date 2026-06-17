use std::collections::HashMap;

use anyhow::Result;
use futures::StreamExt;
use runie_core::llm_event::LLMEvent;
use runie_core::message::ChatMessage;
use runie_core::orchestrator::OrchestratorPlan;
use runie_core::provider::Provider;
use runie_core::trait_resolver::ModelResolver;
use tokio::time::{timeout, Duration};

use crate::planner::config::PlannerConfig;
use crate::planner::error::PlannerError;
use crate::planner::parser::{extract_json_from_text, parse_raw_plan, RawPlan};
use crate::planner::prompt::{build_planner_system_prompt, build_user_prompt};
use crate::planner::types::{PlanInput, ToolDescription};

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

        let mut messages = vec![ChatMessage::system(system), ChatMessage::user(user)];

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
            eprintln!(
                "DEBUG: Trying to parse text (first 100 chars): {:?}",
                &text[..text.len().min(100)]
            );
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
