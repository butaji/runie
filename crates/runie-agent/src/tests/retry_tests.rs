use super::*;
use async_stream::stream;
use futures::stream::BoxStream;
use runie_core::{Event as LlmEvent, ToolSchema, ProviderError};

/// Provider that returns rate limited errors with progressive retries
pub struct RateLimitedProvider {
    call_count: std::sync::atomic::AtomicU32,
}

impl RateLimitedProvider {
    pub fn new() -> Self {
        RateLimitedProvider {
            call_count: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl Provider for RateLimitedProvider {
    fn name(&self) -> &str { "rate_limited" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { false }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, _tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if count < 3 {
            Err(ProviderError::RateLimited)
        } else {
            let s = stream! {
                yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
                yield LlmEvent::MessageDelta { content: "Hello".to_string() };
                yield LlmEvent::MessageEnd;
            };
            Ok(Box::pin(s))
        }
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Provider that returns API error (401-like) immediately without retry
pub struct UnauthorizedProvider;

impl UnauthorizedProvider {
    pub fn new() -> Self { UnauthorizedProvider }
}

#[async_trait]
impl Provider for UnauthorizedProvider {
    fn name(&self) -> &str { "unauthorized" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { false }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, _tools: Vec<ToolSchema>) -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        Err(ProviderError::ApiError("Invalid API key".to_string()))
    }

    async fn chat_simple(&self, _messages: Vec<Message>) -> Result<String, ProviderError> {
        Ok("done".to_string())
    }
}

/// Test that 429 rate limit errors are retried with exponential backoff (1s, 2s, 4s)
#[tokio::test]
#[ignore]
async fn test_provider_429_retry_backoff() {
    use crate::loop_engine::start_chat_with_retry;

    let provider = Arc::new(RateLimitedProvider::new());
    let messages = vec![Message::User { content: "hello".to_string(), attachments: vec![] }];

    let start = std::time::Instant::now();

    let result = start_chat_with_retry(provider, messages, vec![]).await;

    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Should eventually succeed after retries");
    assert!(elapsed.as_secs() >= 7, "Backoff should total at least 7s, got {}s", elapsed.as_secs());
}

/// Test that 401 unauthorized errors fail immediately without retry
#[tokio::test]
async fn test_provider_non_429_no_retry() {
    use crate::loop_engine::start_chat_with_retry;

    let provider = Arc::new(UnauthorizedProvider::new());
    let messages = vec![Message::User { content: "hello".to_string(), attachments: vec![] }];

    let start = std::time::Instant::now();

    let result = start_chat_with_retry(provider, messages, vec![]).await;

    let elapsed = start.elapsed();

    assert!(result.is_err(), "Should fail immediately on non-retryable error");
    assert!(elapsed.as_secs() < 1, "Should fail immediately without backoff, got {}s", elapsed.as_secs());
}
