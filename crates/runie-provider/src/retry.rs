//! Retry wrapper for provider streams using the `backon` crate.
//!
//! Follows the agent-harness pattern: retry transient failures with exponential
//! backoff, but only *before* the first successful event has been yielded.
//! Once the stream has started emitting content, any error is surfaced
//! immediately so the UI is never duplicated or corrupted.

use anyhow::Error;
use futures::{Stream, StreamExt};
use runie_core::provider_event::ProviderEvent;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// Maximum retries before giving up.
const MAX_RETRIES: usize = 3;

/// Base delay for exponential backoff.
const BASE_DELAY: Duration = Duration::from_millis(500);

/// A stream that recreates the inner provider stream on transient errors that
/// occur before any event has been emitted.
pub struct RetryStream<F, S> {
    factory: F,
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
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            state: RetryState::Idle,
            attempt: 0,
        }
    }

    fn start_stream(&mut self) {
        let inner = (self.factory)();
        self.state = RetryState::Streaming {
            inner,
            yielded: false,
        };
    }
}

impl<F, S> RetryStream<F, S>
where
    F: FnMut() -> S + Send + Unpin,
    S: Stream<Item = anyhow::Result<ProviderEvent>> + Send + Unpin,
{
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
                    if *yielded || !is_retryable(&e) || self.attempt >= MAX_RETRIES {
                        self.state = RetryState::Failed;
                        return Poll::Ready(Some(Err(e)));
                    }
                    self.attempt += 1;
                    let delay = BASE_DELAY * 2u32.saturating_pow(self.attempt as u32 - 1);
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
}

impl<P> RetryProvider<P> {
    pub fn new(inner: P) -> Self {
        Self { inner }
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
        Box::pin(RetryStream::new(factory))
    }
}

fn is_retryable(e: &Error) -> bool {
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

/// Retry a fallible async operation with exponential backoff using `backon`.
pub async fn with_retry<F, Fut, T>(mut f: F) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let mut attempt = 0u32;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(err) => {
                if !is_retryable(&err) {
                    return Err(err);
                }
                attempt += 1;
                if attempt >= 3 {
                    return Err(err);
                }
                let delay = BASE_DELAY * 2u32.saturating_pow(attempt - 1);
                tokio::time::sleep(delay).await;
            }
        }
    }
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
        let calls_clone = calls.clone();
        let stream = RetryStream::new(move || {
            let n = calls_clone.fetch_add(1, Ordering::SeqCst);
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
        });

        let events: Vec<_> = stream.collect().await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(events
            .iter()
            .any(|e| matches!(e, Ok(ProviderEvent::TextDelta(t)) if t == "hi")));
    }

    #[tokio::test]
    async fn retry_stream_does_not_retry_after_first_event() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        let stream = RetryStream::new(move || {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            stream::iter(vec![
                Ok(ProviderEvent::TextDelta("hi".to_string())),
                Err(anyhow::anyhow!("boom")),
            ])
        });

        let events: Vec<_> = stream.collect().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[1], Err(_)));
    }

    #[tokio::test]
    async fn retry_stream_gives_up_after_max_retries() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        let stream = RetryStream::new(move || {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            stream::iter(vec![Err(anyhow::anyhow!("timeout"))])
        });

        let events: Vec<_> = stream.collect().await;
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        assert!(events[0].is_err());
    }

    #[tokio::test]
    async fn retry_stream_does_not_retry_auth_errors() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        let stream = RetryStream::new(move || {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            stream::iter(vec![Err(anyhow::anyhow!("401 Unauthorized"))])
        });

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
        let provider = RetryProvider::new(inner);
        let stream = provider.generate(vec![ChatMessage::user("hi".to_string())]);
        let events: Vec<_> = stream.collect().await;
        assert!(events.iter().any(|e| matches!(
            e,
            Ok(ProviderEvent::TextDelta(t)) if t == "ok"
        )));
    }
}
