use async_trait::async_trait;
use runie_core::{Message, ToolSchema, Event, ProviderError};
use futures::stream::BoxStream;

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;

    fn supports_tools(&self) -> bool {
        false
    }

    fn supports_vision(&self) -> bool {
        false
    }

    fn max_context_tokens(&self) -> usize {
        128_000
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError>;

    async fn chat_simple(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, ProviderError>;
}
