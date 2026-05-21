use async_trait::async_trait;
use tidy_core::{Message, ToolSchema, Event, ProviderError};
use futures::stream::BoxStream;

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;

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
