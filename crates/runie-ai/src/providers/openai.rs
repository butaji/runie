use async_trait::async_trait;
use async_stream::stream;
use chrono::Utc;
use futures::stream::BoxStream;
use reqwest::Client;
use runie_core::{Event, Message, ProviderError, ToolSchema};
use serde::Deserialize;
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
                                        "arguments": tc.arguments
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
                                        for tc_delta in tool_calls {
                                            if let Some(name) = tc_delta.function.as_ref().and_then(|f| f.name.clone()) {
                                                let args = tc_delta.function.and_then(|f| f.arguments).unwrap_or_default();
                                                yield Event::ToolCallDelta {
                                                    name,
                                                    arguments: args,
                                                };
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
