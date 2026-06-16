//! OpenAI provider — streams Chat Completions via SSE

use anyhow::Result;
use futures::Stream;
use futures::StreamExt;
use runie_core::llm_event::{LLMEvent, StopReason};
use runie_core::provider::{Message, Provider};
use std::pin::Pin;

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: impl Into<String>) -> Self {
        Self {
            api_key,
            model: model.into(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

enum SseEvent {
    Content(String),
    Done,
}

fn parse_sse_event(line: &str) -> Option<SseEvent> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return Some(SseEvent::Done);
    }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    let content = json
        .get("choices")?
        .get(0)?
        .get("delta")?
        .get("content")?
        .as_str()?;
    Some(SseEvent::Content(content.to_string()))
}

async fn send_openai_request(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    base_url: &str,
    messages: &[Message],
) -> Result<reqwest::Response> {
    let url = format!("{}/chat/completions", base_url);
    let body = serde_json::json!({
        "model": model,
        "messages": messages.iter().map(|m| serde_json::json!({
            "role": m.role(),
            "content": m.content(),
        })).collect::<Vec<_>>(),
        "stream": true,
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("OpenAI request failed: {}", e))?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("OpenAI error: {}", text));
    }
    Ok(response)
}

fn openai_stream(
    api_key: String,
    model: String,
    base_url: String,
    messages: Vec<Message>,
) -> Pin<Box<dyn Stream<Item = Result<LLMEvent>> + Send>> {
    Box::pin(async_stream::stream! {
        let client = reqwest::Client::new();
        let response = match send_openai_request(&client, &api_key, &model, &base_url, &messages).await {
            Ok(r) => r,
            Err(e) => {
                yield Err(e);
                return;
            }
        };
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk));
                    let events = drain_buffer(&mut buffer);
                    for event in events {
                        yield Ok(event);
                    }
                }
                Err(e) => {
                    yield Err(anyhow::anyhow!("SSE stream error: {}", e));
                    return;
                }
            }
        }
    })
}

fn drain_buffer(buffer: &mut String) -> Vec<LLMEvent> {
    let mut events = Vec::new();
    while let Some(pos) = buffer.find('\n') {
        let line = buffer[..pos].trim().to_string();
        *buffer = buffer[pos + 1..].to_string();
        match parse_sse_event(&line) {
            Some(SseEvent::Done) => {
                events.push(LLMEvent::Finish { reason: StopReason::Stop });
                break;
            }
            Some(SseEvent::Content(content)) => {
                events.push(LLMEvent::TextDelta(content));
            }
            None => {}
        }
    }
    events
}

impl Provider for OpenAiProvider {
    fn generate(
        &self,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = Result<LLMEvent>> + Send + '_>> {
        openai_stream(
            self.api_key.clone(),
            self.model.clone(),
            self.base_url.clone(),
            messages,
        )
    }
}
