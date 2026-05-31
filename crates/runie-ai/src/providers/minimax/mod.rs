use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use runie_core::{Event, Message, ProviderError, ToolSchema};

use crate::Provider;

pub mod stream;
pub mod types;

const HTTP_TIMEOUT_SECS: u64 = 120;
const HTTP_CONNECT_TIMEOUT_SECS: u64 = 30;

pub struct MiniMaxProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: Client,
}

impl MiniMaxProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
            .connect_timeout(std::time::Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            api_key: api_key.trim().to_string(),
            model,
            base_url: "https://api.minimax.io/v1".to_string(),
            client,
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url.trim_end_matches('/').to_string();
        self
    }

    pub fn cost_per_1k_tokens(&self) -> (f64, f64) {
        match self.model.as_str() {
            "abab5.5s" | "abab5.5g" => (0.0006, 0.002),
            "abab5.5-chat" => (0.001, 0.002),
            "abab6" | "abab6.5s" => (0.001, 0.002),
            "abab6.5g" => (0.001, 0.004),
            _ => (0.001, 0.002),
        }
    }

    fn messages_to_minimax(&self, messages: Vec<Message>) -> Vec<serde_json::Value> {
        messages.into_iter().map(|msg| match msg {
            Message::System { content } => serde_json::json!({ "role": "system", "content": content }),
            Message::User { content, .. } => serde_json::json!({ "role": "user", "content": content }),
            Message::Assistant { content, tool_calls, .. } => {
                let mut obj = serde_json::json!({ "role": "assistant", "content": content });
                if !tool_calls.is_empty() {
                    let calls: Vec<serde_json::Value> = tool_calls.into_iter().map(|tc| {
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default()
                            }
                        })
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

    fn tools_to_minimax(&self, tools: Vec<ToolSchema>) -> Vec<serde_json::Value> {
        tools.into_iter().map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters
                }
            })
        }).collect()
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
                    let delay = std::time::Duration::from_secs(2u64.pow(attempt));
                    tracing::warn!("Rate limited, retrying in {}s...", delay.as_secs());
                    tokio::time::sleep(delay).await;
                }
                Err(e) => { last_error = Some(e); break; }
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
        tracing::debug!("[MiniMax] Request URL: {}", url);
        tracing::debug!("[MiniMax] Model: {}", self.model);
        tracing::debug!("[MiniMax] API Key prefix: {}...", &self.api_key[..self.api_key.len().min(8)]);
        let minimax_messages = self.messages_to_minimax(messages.clone());
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": minimax_messages,
            "stream": true
        });

        if has_tools {
            body["tools"] = serde_json::json!(self.tools_to_minimax(tools));
            body["tool_choice"] = serde_json::json!("auto");
        }

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 429 { return Err(ProviderError::RateLimited); }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|e| format!("(body error: {})", e));
            let error_msg = if status.as_u16() == 401 && body.contains("1004") {
                format!(
                    "{}: {}\n\nMiniMax requires an API Secret Key (not an API Key). \
                     Please use your API Secret Key from the MiniMax console.",
                    status, body
                )
            } else {
                format!("{}: {}", status, body)
            };
            return Err(ProviderError::ApiError(error_msg));
        }

        stream::build_minimax_stream(response, messages).await
    }

    async fn chat_non_streaming(&self, messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<String, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let minimax_messages = self.messages_to_minimax(messages);
        let has_tools = !tools.is_empty();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": minimax_messages,
            "stream": false
        });

        if has_tools {
            body["tools"] = serde_json::json!(self.tools_to_minimax(tools));
            body["tool_choice"] = serde_json::json!("auto");
        }

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;

        let status = response.status();
        if status.as_u16() == 429 { return Err(ProviderError::RateLimited); }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|e| format!("(body error: {})", e));
            let error_msg = if status.as_u16() == 401 && body.contains("1004") {
                format!(
                    "{}: {}\n\nMiniMax requires an API Secret Key (not an API Key). \
                     Please use your API Secret Key from the MiniMax console.",
                    status, body
                )
            } else {
                format!("{}: {}", status, body)
            };
            return Err(ProviderError::ApiError(error_msg));
        }

        let result: types::MiniMaxResponse = response.json().await.map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        Ok(result.choices.into_iter().next().and_then(|c| c.message.content).unwrap_or_default())
    }
}

#[async_trait]
impl Provider for MiniMaxProvider {
    fn name(&self) -> &str { "minimax" }
    fn model(&self) -> &str { &self.model }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, messages: Vec<Message>, tools: Vec<ToolSchema>) -> Result<BoxStream<'static, Event>, ProviderError> {
        self.chat_with_retry(messages, tools).await
    }

    async fn chat_simple(&self, messages: Vec<Message>) -> Result<String, ProviderError> {
        self.chat_non_streaming(messages, vec![]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::ToolCall;

    #[test]
    fn test_tool_call_arguments_serialized_as_string() {
        let provider = MiniMaxProvider::new("test-key".to_string(), "minimax".to_string());
        let messages = vec![Message::Assistant {
            content: "".to_string(),
            tool_calls: vec![ToolCall { id: "call_123".to_string(), name: "bash".to_string(), arguments: serde_json::json!({"command": "ls -la"}) }],
            thinking: None,
        }];
        let json_messages = provider.messages_to_minimax(messages);
        let args = &json_messages[0]["tool_calls"][0]["function"]["arguments"];
        assert!(args.is_string(), "arguments should be a string");
        assert_eq!(args.as_str().unwrap(), "{\"command\":\"ls -la\"}");
    }
}
