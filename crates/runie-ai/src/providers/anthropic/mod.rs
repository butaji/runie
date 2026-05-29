use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use runie_core::{Event, Message, ProviderError, ToolSchema};
use std::env;

use crate::Provider;

pub mod stream;
pub mod types;

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: Client,
    max_tokens: usize,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let api_key = if api_key.is_empty() { env::var("ANTHROPIC_API_KEY").unwrap_or_default() } else { api_key };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { api_key, model, base_url: "https://api.anthropic.com/v1".to_string(), client, max_tokens: 4096 }
    }

    pub fn with_max_tokens(mut self, tokens: usize) -> Self { self.max_tokens = tokens; self }
    pub fn with_base_url(mut self, url: String) -> Self { self.base_url = url.trim_end_matches('/').to_string(); self }

    pub fn cost_per_1k_tokens(&self) -> (f64, f64) {
        match self.model.as_str() {
            "claude-3-5-sonnet" | "claude-3-5-sonnet-20241022" | "claude-sonnet-4-20250514" => (0.003, 0.015),
            "claude-3-opus" | "claude-opus-4-20250514" => (0.015, 0.075),
            "claude-3-haiku" | "claude-haiku-4-20250514" => (0.0003, 0.00125),
            "claude-3-sonnet" => (0.003, 0.015),
            "claude-3-5-haiku" => (0.0008, 0.004),
            _ => (0.003, 0.015),
        }
    }

    fn messages_to_anthropic(&self, messages: Vec<Message>) -> (Vec<serde_json::Value>, Option<String>) {
        let mut system_content = None;
        let anthropic_messages: Vec<serde_json::Value> = messages.into_iter().filter_map(|msg| match msg {
            Message::System { content } => { system_content = Some(content); None }
            Message::User { content, .. } => Some(serde_json::json!({ "role": "user", "content": content })),
            Message::Assistant { content, tool_calls, .. } => {
                let mut obj = serde_json::json!({ "role": "assistant", "content": content });
                if !tool_calls.is_empty() {
                    let calls: Vec<serde_json::Value> = tool_calls.into_iter().map(|tc| {
                        serde_json::json!({ "id": tc.id, "type": "tool_use", "name": tc.name, "input": tc.arguments })
                    }).collect();
                    obj["tool_calls"] = serde_json::json!(calls);
                }
                Some(obj)
            }
            Message::ToolResult { tool_call_id, content, .. } => {
                Some(serde_json::json!({ "role": "user", "content": [{ "type": "tool_result", "tool_use_id": tool_call_id, "content": content }] }))
            }
        }).collect();
        (anthropic_messages, system_content)
    }

    fn tools_to_anthropic(&self, tools: Vec<ToolSchema>) -> Vec<serde_json::Value> {
        tools.into_iter().map(|t| {
            serde_json::json!({ "name": t.name, "description": t.description, "input_schema": t.parameters })
        }).collect()
    }

    async fn chat_with_retry(&self, messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, Event>, ProviderError> {
        let mut last_error = None;
        for attempt in 0..3 {
            match self.chat_once(messages.clone(), tools.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(ProviderError::RateLimited) => {
                    let delay = tokio::time::Duration::from_secs(2u64.pow(attempt));
                    tracing::warn!("Rate limited, retrying in {}s...", delay.as_secs());
                    tokio::time::sleep(delay).await;
                }
                Err(e) => { last_error = Some(e); break; }
            }
        }
        Err(last_error.unwrap_or(ProviderError::ApiError("Max retries exceeded".to_string())))
    }

    async fn chat_once(&self, messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, Event>, ProviderError> {
        let url = format!("{}/messages", self.base_url);
        let (anthropic_messages, system) = self.messages_to_anthropic(messages.clone());
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({ "model": self.model, "messages": anthropic_messages, "max_tokens": self.max_tokens, "stream": true });
        if let Some(sys) = system { body["system"] = serde_json::json!(sys); }
        if has_tools { body["tools"] = serde_json::json!(self.tools_to_anthropic(tools)); }

        let response = self.client.post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body).send().await.map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 401 { return Err(ProviderError::ApiError("Invalid API key".to_string())); }
        if status.as_u16() == 429 { return Err(ProviderError::RateLimited); }
        if status.as_u16() == 529 { return Err(ProviderError::ApiError("Anthropic service overloaded".to_string())); }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|e| format!("(body error: {})", e));
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        stream::build_anthropic_stream(response).await
    }

    async fn chat_non_streaming(&self, messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<String, ProviderError> {
        let url = format!("{}/messages", self.base_url);
        let (anthropic_messages, system) = self.messages_to_anthropic(messages);
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({ "model": self.model, "messages": anthropic_messages, "max_tokens": self.max_tokens, "stream": false });
        if let Some(sys) = system { body["system"] = serde_json::json!(sys); }
        if has_tools { body["tools"] = serde_json::json!(self.tools_to_anthropic(tools)); }

        let response = self.client.post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json").json(&body).send().await.map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 401 { return Err(ProviderError::ApiError("Invalid API key".to_string())); }
        if status.as_u16() == 429 { return Err(ProviderError::RateLimited); }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|e| format!("(body error: {})", e));
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        let result: types::AnthropicResponse = response.json().await.map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        let content = result.content.into_iter().next().and_then(|c| if c.type_ == "text" { c.text } else { None }).unwrap_or_default();
        Ok(content)
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str { "anthropic" }
    fn model(&self) -> &str { &self.model }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { self.model.contains("claude-3") }
    fn max_context_tokens(&self) -> usize { 200_000 }
    async fn chat(&self, messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, Event>, ProviderError> { self.chat_with_retry(messages, tools).await }
    async fn chat_simple(&self, messages: Vec<Message>) -> Result<String, ProviderError> { self.chat_non_streaming(messages, vec![]).await }
}
