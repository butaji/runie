use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use runie_core::{Event, Message, ProviderError, ToolSchema};
use std::env;

use crate::Provider;

pub mod stream;
pub mod types;

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
        let api_key = if api_key.is_empty() { env::var("OPENAI_API_KEY").unwrap_or_default() } else { api_key };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
            .connect_timeout(std::time::Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { api_key, model, base_url: "https://api.openai.com/v1".to_string(), client }
    }

    pub fn with_base_url(mut self, url: String) -> Self { self.base_url = url.trim_end_matches('/').to_string(); self }

    pub fn cost_per_1k_tokens(&self) -> (f64, f64) {
        match self.model.as_str() {
            "gpt-4o" | "gpt-4o-2024-08-06" | "o1-preview" | "o1-mini" => (0.005, 0.015),
            "gpt-4" | "gpt-4-turbo" | "gpt-4-turbo-2024-04-09" => (0.03, 0.06),
            "gpt-3.5-turbo" => (0.0005, 0.0015),
            _ => (0.0, 0.0),
        }
    }

    fn messages_to_openai(&self, messages: Vec<Message>) -> Vec<serde_json::Value> {
        messages.into_iter().map(|msg| match msg {
            Message::System { content } => serde_json::json!({ "role": "system", "content": content }),
            Message::User { content, .. } => serde_json::json!({ "role": "user", "content": content }),
            Message::Assistant { content, tool_calls, .. } => {
                let mut obj = serde_json::json!({ "role": "assistant", "content": content });
                if !tool_calls.is_empty() {
                    let calls: Vec<serde_json::Value> = tool_calls.into_iter().map(|tc| {
                        serde_json::json!({ "id": tc.id, "type": "function", "function": { "name": tc.name, "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default() } })
                    }).collect();
                    obj["tool_calls"] = serde_json::json!(calls);
                }
                obj
            }
            Message::ToolResult { tool_call_id, content, .. } => {
                serde_json::json!({ "role": "tool", "tool_call_id": tool_call_id, "content": content })
            }
        }).collect()
    }

    fn tools_to_openai(&self, tools: Vec<ToolSchema>) -> Vec<serde_json::Value> {
        tools.into_iter().map(|t| {
            serde_json::json!({ "type": "function", "function": { "name": t.name, "description": t.description, "parameters": t.parameters } })
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
        let url = format!("{}/chat/completions", self.base_url);
        let openai_messages = self.messages_to_openai(messages.clone());
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({ "model": self.model, "messages": openai_messages, "stream": true });
        if has_tools { body["tools"] = serde_json::json!(self.tools_to_openai(tools)); body["tool_choice"] = serde_json::json!("auto"); }

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body).send().await.map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 429 { return Err(ProviderError::RateLimited); }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|e| format!("(body error: {})", e));
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        stream::build_openai_stream(response, messages).await
    }

    async fn chat_non_streaming(&self, messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<String, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let openai_messages = self.messages_to_openai(messages);
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({ "model": self.model, "messages": openai_messages, "stream": false });
        if has_tools { body["tools"] = serde_json::json!(self.tools_to_openai(tools)); body["tool_choice"] = serde_json::json!("auto"); }

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json").json(&body).send().await.map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 429 { return Err(ProviderError::RateLimited); }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|e| format!("(body error: {})", e));
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        let result: types::OpenAIResponse = response.json().await.map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        Ok(result.choices.into_iter().next().and_then(|c| c.message.content).unwrap_or_default())
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str { "openai" }
    fn model(&self) -> &str { &self.model }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { self.model.starts_with("gpt-4o") }
    fn max_context_tokens(&self) -> usize {
        if self.model.starts_with("gpt-4o") { 128_000 }
        else if self.model.starts_with("gpt-4") { 8_192 }
        else { 4_096 }
    }
    async fn chat(&self, messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, Event>, ProviderError> { self.chat_with_retry(messages, tools).await }
    async fn chat_simple(&self, messages: Vec<Message>) -> Result<String, ProviderError> { self.chat_non_streaming(messages, vec![]).await }
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
            tool_calls: vec![ToolCall { id: "call_123".to_string(), name: "bash".to_string(), arguments: serde_json::json!({"command": "ls -la"}) }],
            thinking: None,
        }];
        let json_messages = provider.messages_to_openai(messages);
        let args = &json_messages[0]["tool_calls"][0]["function"]["arguments"];
        assert!(args.is_string(), "arguments should be a string");
        assert_eq!(args.as_str().unwrap(), "{\"command\":\"ls -la\"}");
    }
}
