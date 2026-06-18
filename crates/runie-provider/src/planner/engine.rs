use std::collections::HashMap;

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
        let mut messages = self.build_messages(input);
        let mut last_error = String::new();

        for attempt in 1..=self.config.max_retries + 1 {
            let text = self.collect_stream_text(&messages).await?;
            log_parse_attempt(&text);

            match try_parse_plan(&text, &self.tool_names()) {
                Ok(plan) => return Ok(plan),
                Err(e) => last_error = e,
            }

            if attempt <= self.config.max_retries {
                append_correction(&mut messages, &last_error);
            }
        }

        Err(PlannerError::ParseFailed {
            attempts: self.config.max_retries + 1,
            last_error,
        })
    }

    fn build_messages(&self, input: &PlanInput<'_>) -> Vec<ChatMessage> {
        let system = build_planner_system_prompt(input.available_traits, self.tools);
        let user = build_user_prompt(input);
        vec![ChatMessage::system(system), ChatMessage::user(user)]
    }

    async fn collect_stream_text(&self, messages: &[ChatMessage]) -> Result<String, PlannerError> {
        let mut stream = self.provider.generate(messages.to_vec());

        match timeout(self.config.timeout, stream.next()).await {
            Ok(Some(Ok(LLMEvent::TextDelta(initial)))) => {
                Ok(collect_text_deltas(&mut stream, initial).await)
            }
            Ok(Some(Ok(LLMEvent::ThinkingDelta(_)))) => {
                Ok(collect_text_deltas(&mut stream, String::new()).await)
            }
            Ok(Some(Ok(_))) => Ok(String::new()),
            Ok(Some(Err(e))) => Err(PlannerError::ProviderError(e.to_string())),
            Ok(None) => Ok(String::new()),
            Err(_) => Err(PlannerError::Timeout),
        }
    }
}

async fn collect_text_deltas<S>(stream: &mut S, mut text: String) -> String
where
    S: StreamExt<Item = anyhow::Result<LLMEvent>> + Unpin,
{
    while let Some(event) = stream.next().await {
        if let Ok(LLMEvent::TextDelta(delta)) = event {
            text.push_str(&delta);
        }
    }
    text
}

fn log_parse_attempt(text: &str) {
    eprintln!(
        "DEBUG: Trying to parse text (first 100 chars): {:?}",
        &text[..text.len().min(100)]
    );
}

fn try_parse_plan(
    text: &str,
    tool_names: &HashMap<String, ()>,
) -> Result<OrchestratorPlan, String> {
    if let Ok(plan) = parse_raw_plan_text(text, tool_names) {
        return Ok(plan);
    }
    parse_raw_plan_markdown(text, tool_names)
        .map_err(|e| format!("{}; also failed markdown extract", e))
}

fn parse_raw_plan_text(
    text: &str,
    tool_names: &HashMap<String, ()>,
) -> Result<OrchestratorPlan, String> {
    serde_json::from_str::<RawPlan>(text)
        .map_err(|e| e.to_string())
        .and_then(|raw| parse_raw_plan(raw, tool_names).map_err(|e| e.to_string()))
}

fn parse_raw_plan_markdown(
    text: &str,
    tool_names: &HashMap<String, ()>,
) -> Result<OrchestratorPlan, String> {
    let json_text =
        extract_json_from_text(text).ok_or_else(|| "no markdown code block".to_string())?;
    serde_json::from_str::<RawPlan>(&json_text)
        .map_err(|e| e.to_string())
        .and_then(|raw| parse_raw_plan(raw, tool_names).map_err(|e| e.to_string()))
}

fn append_correction(messages: &mut Vec<ChatMessage>, last_error: &str) {
    let correction = format!(
        "\n\n[Planner] Your previous output was not valid JSON: {}. \
         Please respond with ONLY the JSON plan object, no explanation.",
        last_error
    );
    messages.push(ChatMessage::user(correction));
}
