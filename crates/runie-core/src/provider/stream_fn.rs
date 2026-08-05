//! `StreamFn` trait — abstract LLM streaming interface.
//!
//! Adapters implement this for concrete providers (Anthropic, OpenAI,
//! Bedrock, etc.). The agent loop calls `stream` exactly once per assistant
//! turn; events arrive on the returned stream.

use std::pin::Pin;

use futures::Stream;

use crate::types::{AssistantMessageEvent, Model, SimpleStreamOptions};

pub type AssistantMessageEventStream = Pin<Box<dyn Stream<Item = AssistantMessageEvent> + Send>>;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("network: {0}")]
    Network(String),
    #[error("api: {0}")]
    Api(String),
    #[error("aborted")]
    Aborted,
    #[error("invalid: {0}")]
    Invalid(String),
}

#[async_trait::async_trait]
pub trait StreamFn: Send + Sync + 'static {
    async fn stream(
        &self,
        model: &Model,
        context: &crate::types::AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestFn;
    #[async_trait::async_trait]
    impl StreamFn for TestFn {
        async fn stream(
            &self,
            _model: &Model,
            _context: &crate::types::AgentContext,
            _options: Option<SimpleStreamOptions>,
        ) -> Result<AssistantMessageEventStream, StreamError> {
            use futures::stream;
            Ok(Box::pin(stream::empty()))
        }
    }

    #[tokio::test]
    async fn trait_object_works() {
        let _f: std::sync::Arc<dyn StreamFn> = std::sync::Arc::new(TestFn);
    }
}
