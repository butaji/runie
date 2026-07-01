//! Retry configuration for provider streams.
//!
//! Uses `backon` for exponential backoff retry. The retry behavior is:
//! retry transient failures with exponential backoff, but only *before* the
//! stream starts emitting events. Once the stream has started, any error is
//! surfaced immediately.

use crate::ProviderError;
use anyhow::Error;
use backon::{ExponentialBuilder, Retryable};
use futures::Future;

// Re-export from runie-core so callers can use it without importing from runie-core
#[allow(unused_imports)]
pub use crate::ProviderError as classify_reqwest_error;

/// Classify an SSE stream error into a typed `ProviderError` variant.
pub fn from_sse_error(err: &reqwest_eventsource::Error) -> ProviderError {
    use reqwest_eventsource::Error as SseErr;
    match err {
        // UTF-8 decode error — wrap as source
        SseErr::Utf8(_) => {
            ProviderError::Source(anyhow::anyhow!("{err}"))
        }
        // Parser error — wrap as source
        SseErr::Parser(_) => {
            ProviderError::Source(anyhow::anyhow!("{err}"))
        }
        // HTTP-level error from reqwest
        SseErr::Transport(e) => ProviderError::from_reqwest(e),
        // Content-type mismatch
        SseErr::InvalidContentType(_, _) => {
            ProviderError::Source(anyhow::anyhow!("{err}"))
        }
        // HTTP status code error (5xx, 429, 401, 403)
        SseErr::InvalidStatusCode(status, _) => {
            let code = status.as_u16();
            if code == 401 || code == 403 {
                ProviderError::Auth(code)
            } else if code == 429 {
                ProviderError::RateLimit { retry_after_secs: None }
            } else if code >= 500 {
                ProviderError::Server(code, Default::default())
            } else {
                ProviderError::Source(anyhow::anyhow!("{err}"))
            }
        }
        // Invalid Last-Event-ID header
        SseErr::InvalidLastEventId(_) => {
            ProviderError::Source(anyhow::anyhow!("{err}"))
        }
        // Stream ended unexpectedly
        SseErr::StreamEnded => {
            ProviderError::Source(anyhow::anyhow!("SSE stream ended unexpectedly"))
        }
    }
}

/// Determines if an error should trigger a retry using typed `ProviderError`.
/// Falls back to string heuristics for non-`ProviderError` errors.
pub fn is_retryable(e: &Error) -> bool {
    // Fast path: try to classify as a typed ProviderError first
    if let Some(typed) = e.downcast_ref::<ProviderError>() {
        return typed.is_retryable();
    }
    // Also check reqwest errors directly
    if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
        return ProviderError::from_reqwest(reqwest_err).is_retryable();
    }
    // Fallback: string-based heuristics for unknown error shapes
    let msg = e.to_string().to_lowercase();
    msg.contains("timeout")
        || msg.contains("connection")
        || msg.contains("overloaded")
        || msg.contains("rate limit")
        || msg.contains("try again")
}

/// Retry a fallible async operation with exponential backoff using `backon`.
pub async fn with_retry<F, Fut, T>(f: F) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    f.retry(ExponentialBuilder::default())
        .when(is_retryable)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // ── Layer 1: typed ProviderError is_retryable ────────────────────────────────

    #[test]
    fn is_retryable_true_for_typed_rate_limit() {
        // Wrap the typed ProviderError as anyhow::Error so downcast works
        let typed: ProviderError = ProviderError::RateLimit { retry_after_secs: None };
        let err: anyhow::Error = typed.into();
        assert!(is_retryable(&err));
    }

    #[test]
    fn is_retryable_true_for_typed_timeout() {
        let typed: ProviderError = ProviderError::Timeout;
        let err: anyhow::Error = typed.into();
        assert!(is_retryable(&err));
    }

    #[test]
    fn is_retryable_true_for_typed_network() {
        let typed: ProviderError = ProviderError::Network("connection refused".into());
        let err: anyhow::Error = typed.into();
        assert!(is_retryable(&err));
    }

    #[test]
    fn is_retryable_false_for_typed_auth() {
        let typed: ProviderError = ProviderError::Auth(401);
        let err: anyhow::Error = typed.into();
        assert!(!is_retryable(&err));
    }

    #[test]
    fn is_retryable_false_for_typed_context_length() {
        let typed: ProviderError = ProviderError::ContextLength(128_000);
        let err: anyhow::Error = typed.into();
        assert!(!is_retryable(&err));
    }

    #[test]
    fn is_retryable_true_for_typed_server_error() {
        let typed: ProviderError = ProviderError::Server(502, Default::default());
        let err: anyhow::Error = typed.into();
        assert!(is_retryable(&err));
    }

    #[tokio::test]
    async fn with_retry_succeeds_on_first_attempt() {
        let result = with_retry(|| async { Ok::<_, Error>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn with_retry_fails_after_max_attempts() {
        let result: Result<i32, _> =
            with_retry(|| async { Err(anyhow::anyhow!("persistent error")) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn with_retry_retries_transient_errors() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let result = with_retry(move || {
            let c = counter_clone.clone();
            async move {
                let n = c.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Err(anyhow::anyhow!("rate limit"))
                } else {
                    Ok::<_, Error>(42)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        // backon retries with exponential backoff
        assert!(counter.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn retryable_detects_server_errors() {
        let err = anyhow::anyhow!("server overloaded");
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_detects_rate_limit() {
        let err = anyhow::anyhow!("rate limit exceeded");
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_detects_timeout() {
        let err = anyhow::anyhow!("timeout error");
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_detects_connection_error() {
        let err = anyhow::anyhow!("connection refused");
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_rejects_auth_errors() {
        let err = anyhow::anyhow!("401 Unauthorized");
        assert!(!is_retryable(&err));
    }

    #[test]
    fn retryable_rejects_client_errors() {
        let err = anyhow::anyhow!("400 Bad Request");
        assert!(!is_retryable(&err));
    }

    // ── Layer 1: retryable error triggers multiple attempts ───────────────────

    #[tokio::test]
    async fn backon_retries_retryable_error() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c = counter.clone();
        let result: Result<i32, anyhow::Error> = with_retry(move || {
            let c = c.clone();
            async move {
                let n = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n < 2 {
                    Err(anyhow::anyhow!("rate limit"))
                } else {
                    Ok::<_, anyhow::Error>(42)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert!(counter.load(std::sync::atomic::Ordering::SeqCst) >= 2);
    }

    #[tokio::test]
    async fn backon_does_not_retry_fatal_error() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c = counter.clone();
        let result: Result<i32, anyhow::Error> = with_retry(move || {
            let c = c.clone();
            async move {
                c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err(anyhow::anyhow!("401 Unauthorized"))
            }
        })
        .await;
        assert!(result.is_err());
        // Only one attempt for non-retryable error
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
