use async_trait::async_trait;
use async_stream::stream;
use chrono::Utc;
use futures::stream::BoxStream;
use reqwest::Client;
use runie_core::{Event, Message, ProviderError, ToolSchema};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use tokio::time::{sleep, Duration};

use crate::Provider;
use crate::token_usage::TokenUsage;

const HTTP_TIMEOUT_SECS: u64 = 120;
const HTTP_CONNECT_TIMEOUT_SECS: u64 = 30;

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let api_key = if api_key.is_empty() {
            env::var("OPENAI_API_KEY").unwrap_or_default()
        } else {
            api_key
        };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
            .connect_timeout(std::time::Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            api_key,
            model,
            base_url: "https://api.openai.com/v1".to_string(),
            client,
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url.trim_end_matches('/').to_string();
        self
    }

    /// Returns (input_cost, output_cost) per 1K tokens in USD.
    pub fn cost_per_1k_tokens(&self) -> (f64, f64) {
        match self.model.as_str() {
            "gpt-4o" | "gpt-4o-2024-08-06" | "o1-preview" | "o1-mini" => (0.005, 0.015),
            "gpt-4" | "gpt-4-turbo" | "gpt-4-turbo-2024-04-09" => (0.03, 0.06),
            "gpt-3.5-turbo" => (0.0005, 0.0015),
            _ => (0.0, 0.0),
        }
    }

    fn messages_to_openai(&self, messages: Vec<Message>) -> Vec<serde_json::Value> {
        messages
            .into_iter()
            .map(|msg| match msg {
                Message::System { content } => {
                    serde_json::json!({ "role": "system", "content": content })
                }
                Message::User { content, .. } => {
                    serde_json::json!({ "role": "user", "content": content })
                }
                Message::Assistant { content, tool_calls, .. } => {
                    let mut obj = serde_json::json!({ "role": "assistant", "content": content });
                    if !tool_calls.is_empty() {
                        let calls: Vec<serde_json::Value> = tool_calls
                            .into_iter()
                            .map(|tc| {
                                serde_json::json!({
                                    "id": tc.id,
                                    "type": "function",
                                    "function": {
                                        "name": tc.name,
                                        "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default()
                                    }
                                })
                            })
                            .collect();
                        obj["tool_calls"] = serde_json::json!(calls);
                    }
                    obj
                }
                Message::ToolResult { tool_call_id, content, .. } => {
                    serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tool_call_id,
                        "content": content
                    })
                }
            })
            .collect()
    }

    fn tools_to_openai(&self, tools: Vec<ToolSchema>) -> Vec<serde_json::Value> {
        tools
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                })
            })
            .collect()
    }

    async fn chat_with_retry(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        let mut last_error = None;

        for attempt in 0..3 {
            match self.chat_once(messages.clone(), tools.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(ProviderError::RateLimited) => {
                    let delay = Duration::from_secs(2u64.pow(attempt));
                    tracing::warn!("Rate limited, retrying in {}s...", delay.as_secs());
                    sleep(delay).await;
                }
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            }
        }

        Err(last_error.unwrap_or(ProviderError::ApiError("Max retries exceeded".to_string())))
    }

    async fn chat_once(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let openai_messages = self.messages_to_openai(messages.clone());
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": openai_messages,
            "stream": true
        });

        if has_tools {
            body["tools"] = serde_json::json!(self.tools_to_openai(tools));
            body["tool_choice"] = serde_json::json!("auto");
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(ProviderError::RateLimited);
        }
        if !status.is_success() {
            let body = match response.text().await {
                Ok(text) => text,
                Err(e) => format!("(failed to read response body: {})", e),
            };
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        let session_id = format!("openai-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));

        let stream = stream! {
            yield Event::AgentStart { session_id: session_id.clone(), timestamp: Utc::now() };
            yield Event::TurnStart { turn: 0, timestamp: Utc::now() };
            yield Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() };

            let mut text_content = String::new();
            let mut prompt_text = String::new();
            for msg in &messages {
                match msg {
                    Message::System { content } => prompt_text.push_str(content),
                    Message::User { content, .. } => prompt_text.push_str(content),
                    Message::Assistant { content, .. } => prompt_text.push_str(content),
                    Message::ToolResult { content, .. } => prompt_text.push_str(content),
                }
            }

            let mut usage: Option<TokenUsage> = None;
            let mut stream = response.bytes_stream();
            let mut pending_tool_calls: HashMap<usize, PendingToolCall> = HashMap::new();

            use futures::StreamExt;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            if !line.starts_with("data: ") {
                                continue;
                            }
                            let data = &line[6..];
                            if data.trim() == "[DONE]" {
                                continue;
                            }
                            let chunk: OpenAIStreamChunk = match serde_json::from_str(data) {
                                Ok(c) => c,
                                Err(_) => continue,
                            };
                            // Try to capture usage from final chunk
                            if chunk.usage.is_some() {
                                usage = chunk.usage;
                            }
                            if let Some(choice) = chunk.choices.into_iter().next() {
                                if let Some(delta) = choice.delta {
                                    if let Some(c) = delta.content {
                                        text_content.push_str(&c);
                                        yield Event::MessageDelta { content: c };
                                    }
                                    if let Some(tool_calls) = delta.tool_calls {
                                        // Track pending tool calls by index to handle multi-delta tool calls
                                        // where the first delta has id+name and subsequent deltas have partial arguments
                                        // BUG FIX: Don't generate fake IDs when tc_delta.id is None - track by index instead
                                        for tc_delta in tool_calls {
                                            let index = tc_delta.index;
                                            let id = tc_delta.id.clone();
                                            let name = tc_delta.function.as_ref().and_then(|f| f.name.clone());
                                            let args = tc_delta.function.and_then(|f| f.arguments).unwrap_or_default();

                                            if let Some(real_id) = &id {
                                                // We have a real ID - emit accumulated args if we have pending data for this index
                                                if let Some(pending) = pending_tool_calls.remove(&index) {
                                                    // Merge pending args with current args
                                                    let merged_args = format!("{}{}", pending.arguments, args);
                                                    yield Event::ToolCallDelta {
                                                        id: real_id.clone(),
                                                        name: name.unwrap_or_else(|| pending.name),
                                                        arguments: merged_args,
                                                    };
                                                } else {
                                                    // No pending, emit current delta
                                                    if let Some(n) = name {
                                                        yield Event::ToolCallDelta {
                                                            id: real_id.clone(),
                                                            name: n,
                                                            arguments: args,
                                                        };
                                                    }
                                                }
                                            } else if name.is_some() || !args.is_empty() {
                                                // No ID yet - store as pending for this index
                                                // If we already have pending for this index, merge
                                                let entry = pending_tool_calls.entry(index).or_insert_with(|| PendingToolCall {
                                                    name: name.clone().unwrap_or_default(),
                                                    arguments: String::new(),
                                                });
                                                if let Some(n) = &name {
                                                    entry.name = n.clone();
                                                }
                                                entry.arguments.push_str(&args);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Event::Error { message: e.to_string() };
                        break;
                    }
                }
            }

            // Emit usage event
            let (prompt_tokens, completion_tokens, total_tokens) = if let Some(u) = usage {
                (u.prompt_tokens, u.completion_tokens, u.total_tokens)
            } else {
                // Estimate if not provided
                let (pt, ct) = TokenUsage::estimate_from_text(&prompt_text, &text_content);
                (pt, ct, pt + ct)
            };
            yield Event::Usage { prompt_tokens, completion_tokens, total_tokens };

            yield Event::MessageEnd;
            yield Event::AgentEnd { timestamp: Utc::now() };
        };

        Ok(Box::pin(stream))
    }

    async fn chat_non_streaming(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
    ) -> Result<String, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let openai_messages = self.messages_to_openai(messages);
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": openai_messages,
            "stream": false
        });

        if has_tools {
            body["tools"] = serde_json::json!(self.tools_to_openai(tools));
            body["tool_choice"] = serde_json::json!("auto");
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(ProviderError::RateLimited);
        }
        if !status.is_success() {
            let body = match response.text().await {
                Ok(text) => text,
                Err(e) => format!("(failed to read response body: {})", e),
            };
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        let result: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        Ok(result
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default())
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        self.model.starts_with("gpt-4o")
    }

    fn max_context_tokens(&self) -> usize {
        if self.model.starts_with("gpt-4o") {
            128_000
        } else if self.model.starts_with("gpt-4") {
            8_192
        } else {
            4_096
        }
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        self.chat_with_retry(messages, tools).await
    }

    async fn chat_simple(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, ProviderError> {
        self.chat_non_streaming(messages, vec![]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::ToolCall;

    #[test]
    fn test_tool_call_arguments_serialized_as_string() {
        let provider = OpenAiProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![Message::Assistant {
            content: "".to_string(),
            tool_calls: vec![ToolCall {
                id: "call_123".to_string(),
                name: "bash".to_string(),
                arguments: serde_json::json!({"command": "ls -la"}),
            }],
            thinking: None,
        }];
        let json_messages = provider.messages_to_openai(messages);
        let tool_calls = json_messages[0]["tool_calls"].as_array().unwrap();
        let args = &tool_calls[0]["function"]["arguments"];
        assert!(args.is_string(), "arguments should be a string, got: {:?}", args);
        assert_eq!(args.as_str().unwrap(), "{\"command\":\"ls -la\"}");
    }

    #[test]
    fn test_tool_call_with_multiple_args_serialized_correctly() {
        let provider = OpenAiProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![Message::Assistant {
            content: "".to_string(),
            tool_calls: vec![ToolCall {
                id: "call_456".to_string(),
                name: "search".to_string(),
                arguments: serde_json::json!({"query": "rust programming", "limit": 5}),
            }],
            thinking: None,
        }];
        let json_messages = provider.messages_to_openai(messages);
        let tool_calls = json_messages[0]["tool_calls"].as_array().unwrap();
        let args = &tool_calls[0]["function"]["arguments"];
        assert!(args.is_string(), "arguments should be a string");
        let args_str = args.as_str().unwrap();
        assert!(args_str.contains("\"query\":\"rust programming\""));
        assert!(args_str.contains("\"limit\":5"));
    }

    #[test]
    fn test_empty_tool_calls_not_serialized() {
        let provider = OpenAiProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![Message::Assistant {
            content: "Hello".to_string(),
            tool_calls: vec![],
            thinking: None,
        }];
        let json_messages = provider.messages_to_openai(messages);
        assert!(!json_messages[0].as_object().unwrap().contains_key("tool_calls"));
    }
}

// --- OpenAI API types ---

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamChoice {
    delta: Option<Delta>,
    finish_reason: Option<String>,
}

/// Holds pending tool call data when ID is not yet available
/// Used to track multi-delta tool calls where first delta has id+name
/// and subsequent deltas have partial arguments
#[derive(Debug)]
struct PendingToolCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Delta {
    content: Option<String>,
    role: Option<String>,
    tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct ToolCallDelta {
    index: usize,
    id: Option<String>,
    #[serde(rename = "type")]
    type_: Option<String>,
    function: Option<FunctionDelta>,
}

#[derive(Debug, Deserialize, Clone)]
struct FunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageResponse,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MessageResponse {
    role: String,
    content: Option<String>,
}
