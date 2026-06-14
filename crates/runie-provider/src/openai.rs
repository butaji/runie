//! OpenAI provider — streams Chat Completions via SSE

use anyhow::Result;
use futures::Stream;
use futures::StreamExt;
use runie_core::provider::{Message, Provider, ResponseChunk};
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

impl Provider for OpenAiProvider {
    fn generate(
        &self,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = Result<ResponseChunk>> + Send + '_>> {
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let base_url = self.base_url.clone();
        Box::pin(async_stream::stream! {
            let client = reqwest::Client::new();
            let url = format!("{}/chat/completions", base_url);

            let body = serde_json::json!({
                "model": model,
                "messages": messages.iter().map(|m| serde_json::json!({
                    "role": m.role(),
                    "content": m.content(),
                })).collect::<Vec<_>>(),
                "stream": true,
            });

            let response = match client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    yield Err(anyhow::anyhow!("OpenAI request failed: {}", e));
                    return;
                }
            };

            if !response.status().is_success() {
                let text = response.text().await.unwrap_or_default();
                yield Err(anyhow::anyhow!("OpenAI error: {}", text));
                return;
            }

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim().to_string();
                            buffer = buffer[pos + 1..].to_string();
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    return;
                                }
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(content) = json
                                        .get("choices")
                                        .and_then(|c| c.get(0))
                                        .and_then(|c| c.get("delta"))
                                        .and_then(|d| d.get("content"))
                                        .and_then(|c| c.as_str())
                                    {
                                        yield Ok(ResponseChunk { content: content.to_string() });
                                    }
                                }
                            }
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
}
