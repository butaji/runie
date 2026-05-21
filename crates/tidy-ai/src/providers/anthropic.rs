use async_trait::async_trait;
use tidy_core::{Message, ToolSchema, Event, ProviderError};
use futures::stream::BoxStream;
use crate::Provider;

#[allow(dead_code)]
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
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

    async fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        use async_stream::stream;
        let s = stream! {
            yield Event::AgentStart {
                session_id: "stub".to_string(),
                timestamp: chrono::Utc::now()
            };
            yield Event::MessageDelta { content: "Hello from Anthropic stub".to_string() };
            yield Event::AgentEnd { timestamp: chrono::Utc::now() };
        };
        Ok(Box::pin(s))
    }

    async fn chat_simple(
        &self,
        _messages: Vec<Message>,
    ) -> Result<String, ProviderError> {
        Ok("Hello from Anthropic stub".to_string())
    }
}
