use async_trait::async_trait;
use async_stream::stream;
use chrono::Utc;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Client;
use runie_core::{Event, Message, ProviderError, ToolSchema};
use serde::Deserialize;
use std::env;
use tokio::time::{sleep, Duration};

use crate::Provider;

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: Client,
    max_tokens: usize,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let api_key = if api_key.is_empty() {
            env::var("ANTHROPIC_API_KEY").unwrap_or_default()
        } else {
            api_key
        };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            api_key,
            model,
            base_url: "https://api.anthropic.com/v1".to_string(),
            client,
            max_tokens: 4096,
        }
    }

    pub fn with_max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = tokens;
        self
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url.trim_end_matches('/').to_string();
        self
    }

    fn messages_to_anthropic(&self, messages: Vec<Message>) -> (Vec<serde_json::Value>, Option<String>) {
        let mut system_content = None;
        let anthropic_messages: Vec<serde_json::Value> = messages
            .into_iter()
            .filter_map(|msg| match msg {
                Message::System { content } => {
                    system_content = Some(content);
                    None
                }
                Message::User { content, .. } => {
                    Some(serde_json::json!({ "role": "user", "content": content }))
                }
                Message::Assistant { content, tool_calls, .. } => {
                    let mut obj = serde_json::json!({ "role": "assistant", "content": content });
                    if !tool_calls.is_empty() {
                        let calls: Vec<serde_json::Value> = tool_calls
                            .into_iter()
                            .map(|tc| {
                                serde_json::json!({
                                    "id": tc.id,
                                    "type": "tool_use",
                                    "name": tc.name,
                                    "input": tc.arguments
                                })
                            })
                            .collect();
                        obj["tool_calls"] = serde_json::json!(calls);
                    }
                    Some(obj)
                }
                Message::ToolResult { tool_call_id, content, .. } => {
                    Some(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": content
                        }]
                    }))
                }
            })
            .collect();
        (anthropic_messages, system_content)
    }

    fn tools_to_anthropic(&self, tools: Vec<ToolSchema>) -> Vec<serde_json::Value> {
        tools
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters
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
        let url = format!("{}/messages", self.base_url);
        let (anthropic_messages, system) = self.messages_to_anthropic(messages.clone());
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": anthropic_messages,
            "max_tokens": self.max_tokens,
            "stream": true
        });

        if let Some(sys) = system {
            body["system"] = serde_json::json!(sys);
        }

        if has_tools {
            body["tools"] = serde_json::json!(self.tools_to_anthropic(tools));
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 401 {
            return Err(ProviderError::ApiError("Invalid API key".to_string()));
        }
        if status.as_u16() == 429 {
            return Err(ProviderError::RateLimited);
        }
        if status.as_u16() == 529 {
            return Err(ProviderError::ApiError("Anthropic service overloaded".to_string()));
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        let session_id = format!("anthropic-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));

        let stream = stream! {
            yield Event::AgentStart { session_id: session_id.clone(), timestamp: Utc::now() };
            yield Event::TurnStart { turn: 0, timestamp: Utc::now() };
            yield Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() };

            let mut text_content = String::new();
            let mut current_tool_name = String::new();
            let mut current_tool_args = String::new();
            let mut in_tool_block = false;

            let mut stream = response.bytes_stream();

            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            if !line.starts_with("event: ") && !line.starts_with("data: ") {
                                continue;
                            }

                            let (event_type, data) = if let Some(stripped) = line.strip_prefix("event: ") {
                                (stripped.trim(), None)
                            } else if let Some(stripped) = line.strip_prefix("data: ") {
                                ("", Some(stripped.trim()))
                            } else {
                                continue;
                            };

                            if event_type == "message_start" || event_type == "content_block_start"
                               || event_type == "content_block_delta" || event_type == "message_delta"
                               || event_type == "message_stop" || event_type == "content_block_stop" {
                                if let Some(data_str) = data {
                                    let chunk: AnthropicStreamChunk = match serde_json::from_str(data_str) {
                                        Ok(c) => c,
                                        Err(_) => continue,
                                    };

                                    match chunk.clone() {
                                        AnthropicStreamChunk::MessageStart(_) => {},
                                        AnthropicStreamChunk::ContentBlockStart(cb) => {
                                            if cb.type_ == "tool_use" {
                                                current_tool_name = cb.name.unwrap_or_default();
                                                current_tool_args.clear();
                                                in_tool_block = true;
                                            }
                                        }
                                        AnthropicStreamChunk::ContentBlockDelta(delta) => {
                                            match delta.type_.as_str() {
                                                "text_delta" => {
                                                    if let Some(text) = delta.text {
                                                        text_content.push_str(&text);
                                                        yield Event::MessageDelta { content: text };
                                                    }
                                                }
                                                "input_json_delta" => {
                                                    if let Some(partial) = delta.partial_json {
                                                        current_tool_args.push_str(&partial);
                                                        yield Event::ToolCallDelta {
                                                            name: current_tool_name.clone(),
                                                            arguments: current_tool_args.clone(),
                                                        };
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        AnthropicStreamChunk::ContentBlockStop => {
                                            if in_tool_block {
                                                in_tool_block = false;
                                            }
                                        }
                                        AnthropicStreamChunk::MessageDelta(delta) => {
                                            if let Some(usage) = delta.usage {
                                                yield Event::Usage {
                                                    prompt_tokens: usage.input_tokens,
                                                    completion_tokens: usage.output_tokens,
                                                    total_tokens: usage.input_tokens + usage.output_tokens,
                                                };
                                            }
                                        }
                                        AnthropicStreamChunk::MessageStop => {
                                            yield Event::MessageEnd;
                                            yield Event::AgentEnd { timestamp: Utc::now() };
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
        };

        Ok(Box::pin(stream))
    }

    async fn chat_non_streaming(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
    ) -> Result<String, ProviderError> {
        let url = format!("{}/messages", self.base_url);
        let (anthropic_messages, system) = self.messages_to_anthropic(messages);
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": anthropic_messages,
            "max_tokens": self.max_tokens,
            "stream": false
        });

        if let Some(sys) = system {
            body["system"] = serde_json::json!(sys);
        }

        if has_tools {
            body["tools"] = serde_json::json!(self.tools_to_anthropic(tools));
        }

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 401 {
            return Err(ProviderError::ApiError("Invalid API key".to_string()));
        }
        if status.as_u16() == 429 {
            return Err(ProviderError::RateLimited);
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        let result: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let content = result
            .content
            .into_iter()
            .next()
            .and_then(|c| if c.type_ == "text" { c.text } else { None })
            .unwrap_or_default();

        Ok(content)
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        self.model.contains("claude-3")
    }

    fn max_context_tokens(&self) -> usize {
        200_000
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

// --- Anthropic API types ---

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum AnthropicStreamChunk {
    #[serde(rename = "message_start")]
    MessageStart(MessageStartBlock),
    #[serde(rename = "content_block_start")]
    ContentBlockStart(ContentBlockStart),
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta(ContentBlockDelta),
    #[serde(rename = "content_block_stop")]
    ContentBlockStop,
    #[serde(rename = "message_delta")]
    MessageDelta(MessageDelta),
    #[serde(rename = "message_stop")]
    MessageStop,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct MessageStartBlock {
    message: MessageStart,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct MessageStart {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    role: String,
    content: Vec<ContentBlock>,
    model: String,
    #[serde(rename = "stop_reason")]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct ContentBlockStart {
    index: usize,
    #[serde(rename = "type")]
    type_: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct ContentBlockDelta {
    index: usize,
    #[serde(rename = "type")]
    type_: String,
    text: Option<String>,
    #[serde(rename = "partial_json")]
    partial_json: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct MessageDelta {
    #[serde(rename = "type")]
    type_: String,
    usage: Option<DeltaUsage>,
}

#[derive(Debug, Deserialize, Clone)]
struct DeltaUsage {
    #[serde(rename = "input_tokens")]
    input_tokens: usize,
    #[serde(rename = "output_tokens")]
    output_tokens: usize,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct ContentBlock {
    #[serde(rename = "type")]
    type_: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    role: String,
    content: Vec<ResponseContent>,
    model: String,
    #[serde(rename = "stop_reason")]
    stop_reason: Option<String>,
    usage: ResponseUsage,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    #[serde(rename = "type")]
    type_: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResponseUsage {
    #[serde(rename = "input_tokens")]
    input_tokens: usize,
    #[serde(rename = "output_tokens")]
    output_tokens: usize,
}
