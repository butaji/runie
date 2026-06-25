//! Retry wrapper for provider streams.
//!
//! Follows the agent-harness pattern: retry transient failures with exponential
//! backoff, but only *before* the first successful event has been yielded.
//! Once the stream has started emitting content, any error is surfaced
//! immediately so the UI is never duplicated or corrupted.

use futures::{Stream, StreamExt};
use runie_core::provider_event::ProviderEvent;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// Configuration for retrying a provider stream.
#[derive(Debug, Clone, Copy)]
pub struct RetryConfig {
    /// Maximum number of retries before giving up.
    pub max_retries: usize,
    /// Base delay; doubled on each retry.
    pub base_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(500),
        }
    }
}

/// A stream that recreates the inner provider stream on transient errors that
/// occur before any event has been emitted.
pub struct RetryStream<F, S> {
    factory: F,
    config: RetryConfig,
    state: RetryState<S>,
    attempt: usize,
}

enum RetryState<S> {
    Idle,
    Streaming { inner: S, yielded: bool },
    Delaying { sleep: Pin<Box<tokio::time::Sleep>> },
    Failed,
}

impl<F, S> RetryStream<F, S>
where
    F: FnMut() -> S + Send,
    S: Stream<Item = anyhow::Result<ProviderEvent>> + Send,
{
    pub fn new(factory: F, config: RetryConfig) -> Self {
        Self {
            factory,
            config,
            state: RetryState::Idle,
            attempt: 0,
        }
    }

    fn delay_for_attempt(attempt: usize, base: Duration) -> Duration {
        base * 2_u32.pow(attempt as u32)
    }
}

impl<F, S> RetryStream<F, S>
where
    F: FnMut() -> S + Send + Unpin,
    S: Stream<Item = anyhow::Result<ProviderEvent>> + Send + Unpin,
{
    fn start_stream(&mut self) {
        let inner = (self.factory)();
        self.state = RetryState::Streaming {
            inner,
            yielded: false,
        };
    }

    fn poll_delay(&mut self, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
        if let RetryState::Delaying { sleep } = &mut self.state {
            match sleep.as_mut().poll(cx) {
                Poll::Ready(()) => {
                    self.start_stream();
                    cx.waker().wake_by_ref();
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Pending
    }

    fn poll_streaming(&mut self, cx: &mut Context<'_>) -> Poll<Option<<Self as Stream>::Item>> {
        if let RetryState::Streaming { inner, yielded } = &mut self.state {
            match inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(event))) => {
                    *yielded = true;
                    return Poll::Ready(Some(Ok(event)));
                }
                Poll::Ready(Some(Err(e))) => {
                    if *yielded || !is_retryable(&e) || self.attempt >= self.config.max_retries {
                        self.state = RetryState::Failed;
                        return Poll::Ready(Some(Err(e)));
                    }
                    self.attempt += 1;
                    let delay = Self::delay_for_attempt(self.attempt, self.config.base_delay);
                    let deadline = tokio::time::Instant::now() + delay;
                    self.state = RetryState::Delaying {
                        sleep: Box::pin(tokio::time::sleep_until(deadline)),
                    };
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                Poll::Ready(None) => {
                    self.state = RetryState::Failed;
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Ready(None)
    }
}

impl<F, S> Stream for RetryStream<F, S>
where
    F: FnMut() -> S + Send + Unpin,
    S: Stream<Item = anyhow::Result<ProviderEvent>> + Send + Unpin,
{
    type Item = anyhow::Result<ProviderEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match &mut self.state {
                RetryState::Idle => self.start_stream(),
                RetryState::Delaying { .. } => return self.poll_delay(cx),
                RetryState::Streaming { .. } => return self.poll_streaming(cx),
                RetryState::Failed => return Poll::Ready(None),
            }
        }
    }
}

/// A provider wrapper that retries transient stream failures before the first
/// event is emitted.
pub struct RetryProvider<P> {
    inner: P,
    config: RetryConfig,
}

impl<P> RetryProvider<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            config: RetryConfig::default(),
        }
    }

    pub fn with_config(mut self, config: RetryConfig) -> Self {
        self.config = config;
        self
    }
}

impl<P> crate::Provider for RetryProvider<P>
where
    P: crate::Provider + Clone,
{
    fn generate(
        &self,
        messages: Vec<runie_core::message::ChatMessage>,
    ) -> std::pin::Pin<
        Box<
            dyn futures::Stream<Item = anyhow::Result<runie_core::provider_event::ProviderEvent>>
                + Send
                + '_,
        >,
    > {
        let inner = &self.inner;
        let factory = move || inner.generate(messages.clone());
        Box::pin(RetryStream::new(factory, self.config))
    }
}

fn is_retryable(e: &anyhow::Error) -> bool {
    if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
        if let Some(status) = reqwest_err.status() {
            return status.is_server_error() || status == 429;
        }
        return reqwest_err.is_timeout() || reqwest_err.is_connect();
    }
    let msg = e.to_string().to_lowercase();
    msg.contains("timeout")
        || msg.contains("connection")
        || msg.contains("overloaded")
        || msg.contains("rate limit")
        || msg.contains("try again")
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use runie_core::message::ChatMessage;
    use crate::Provider;
    use runie_core::provider_event::StopReason;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn retry_stream_retries_transient_error_before_first_event() {
        let calls = Arc::new(AtomicUsize::new(0));
        let stream = RetryStream::new(
            {
                let calls = calls.clone();
                move || {
                    let n = calls.fetch_add(1, Ordering::SeqCst);
                    if n == 0 {
                        stream::iter(vec![Err(anyhow::anyhow!("rate limit"))])
                    } else {
                        stream::iter(vec![
                            Ok(ProviderEvent::TextDelta("hi".to_string())),
                            Ok(ProviderEvent::Finish {
                                reason: runie_core::provider_event::StopReason::Stop,
                            }),
                        ])
                    }
                }
            },
            RetryConfig {
                max_retries: 2,
                base_delay: Duration::from_millis(1),
            },
        );

        let events: Vec<_> = stream.collect().await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(events
            .iter()
            .any(|e| matches!(e, Ok(ProviderEvent::TextDelta(t)) if t == "hi")));
    }

    #[tokio::test]
    async fn retry_stream_does_not_retry_after_first_event() {
        let calls = Arc::new(AtomicUsize::new(0));
        let stream = RetryStream::new(
            {
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    stream::iter(vec![
                        Ok(ProviderEvent::TextDelta("hi".to_string())),
                        Err(anyhow::anyhow!("boom")),
                    ])
                }
            },
            RetryConfig {
                max_retries: 2,
                base_delay: Duration::from_millis(1),
            },
        );

        let events: Vec<_> = stream.collect().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[1], Err(_)));
    }

    #[tokio::test]
    async fn retry_stream_gives_up_after_max_retries() {
        let calls = Arc::new(AtomicUsize::new(0));
        let stream = RetryStream::new(
            {
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    stream::iter(vec![Err(anyhow::anyhow!("timeout"))])
                }
            },
            RetryConfig {
                max_retries: 2,
                base_delay: Duration::from_millis(1),
            },
        );

        let events: Vec<_> = stream.collect().await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(events.len(), 1);
        assert!(events[0].is_err());
    }

    #[tokio::test]
    async fn retry_stream_does_not_retry_auth_errors() {
        let calls = Arc::new(AtomicUsize::new(0));
        let stream = RetryStream::new(
            {
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    stream::iter(vec![Err(anyhow::anyhow!("401 Unauthorized"))])
                }
            },
            RetryConfig {
                max_retries: 2,
                base_delay: Duration::from_millis(1),
            },
        );

        let events: Vec<_> = stream.collect().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(events[0].is_err());
    }

    #[derive(Clone)]
    struct FlakyProvider {
        calls: Arc<AtomicUsize>,
    }

    impl Provider for FlakyProvider {
        fn generate(
            &self,
            _messages: Vec<ChatMessage>,
        ) -> Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>>
        {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Box::pin(stream::iter(vec![Err(anyhow::anyhow!("timeout"))]))
            } else {
                Box::pin(stream::iter(vec![
                    Ok(ProviderEvent::TextDelta("ok".to_string())),
                    Ok(ProviderEvent::Finish {
                        reason: StopReason::Stop,
                    }),
                ]))
            }
        }
    }

    #[tokio::test]
    async fn retry_provider_retries_failed_generate() {
        let inner = FlakyProvider {
            calls: Arc::new(AtomicUsize::new(0)),
        };
        let provider = RetryProvider::new(inner).with_config(RetryConfig {
            max_retries: 1,
            base_delay: Duration::from_millis(1),
        });
        let stream = provider.generate(vec![ChatMessage::user("hi".to_string())]);
        let events: Vec<_> = stream.collect().await;
        assert!(events.iter().any(|e| matches!(
            e,
            Ok(ProviderEvent::TextDelta(t)) if t == "ok"
        )));
    }
}
