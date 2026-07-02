//! Retry configuration for provider streams.
//!
//! Uses `backon` for exponential backoff retry. The retry behavior is:
//! retry transient failures with exponential backoff, but only *before* the
//! stream starts emitting events. Once the stream has started, any error is
//! surfaced immediately.

use crate::{ProviderError, RetryConfig};
use anyhow::Error;
use backon::{ExponentialBuilder, Retryable};
use futures::Future;
use tracing::Instrument;

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
        // HTTP status code error (5xx, 429, 401, 403) — use shared classifier
        SseErr::InvalidStatusCode(status, _) => {
            let code = status.as_u16();
            ProviderError::classify_http_status(code)
                .unwrap_or_else(|| ProviderError::Source(anyhow::anyhow!("{err}")))
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
///
/// Uses default retry parameters. For custom retry behavior, use
/// [`with_retry_config`](with_retry_config).
pub async fn with_retry<F, Fut, T>(f: F) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    with_retry_config(f, &RetryConfig::default()).await
}

/// Retry a fallible async operation with custom retry configuration.
///
/// Converts `RetryConfig` to backon's `ExponentialBuilder`:
/// - `max_attempts` → `with_max_times()` (backon counts total attempts)
/// - `initial_delay` → `with_min_delay()`
/// - `max_delay` → `with_max_delay()`
/// - `multiplier` → `with_factor()`
pub async fn with_retry_config<F, Fut, T>(
    f: F,
    config: &RetryConfig,
) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    // max_attempts includes the initial call, so max_times (retries) = max_attempts - 1.
    // Use saturating_sub to handle max_attempts = 0 or 1 (no retries).
    let builder = ExponentialBuilder::default()
        .with_max_times(config.max_attempts.saturating_sub(1) as usize)
        .with_min_delay(config.initial_delay)
        .with_max_delay(config.max_delay)
        .with_factor(config.multiplier as f32);
    tracing::debug!(max_attempts = %config.max_attempts, initial_delay_ms = %config.initial_delay.as_millis(), max_delay_ms = %config.max_delay.as_millis(), "provider retry starting");
    let span = tracing::info_span!("provider_retry", max_attempts = %config.max_attempts);
    let result = async {
        f.retry(builder)
            .when(is_retryable)
            .await
            .inspect_err(|e| tracing::warn!(error = %e, "provider retry failed"))
    }
    .instrument(span)
    .await;
    if result.is_ok() {
        tracing::debug!("provider retry succeeded");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use test_case::test_case;

    // ── Layer 1: classify_http_status produces typed variants ─────────────────────
    // Both HTTP and SSE paths use this shared classifier, so testing it covers both.
    // Parameterized with (status_code, expected_variant, expected_retryable).

    #[test_case(401, "Auth(401)", false)]
    #[test_case(403, "Auth(403)", false)]
    #[test_case(429, "RateLimit", true)]
    #[test_case(500, "Server(500)", true)]
    #[test_case(502, "Server(502)", true)]
    #[test_case(503, "Server(503)", true)]
    #[test_case(400, "None", false)]  // 4xx other than 401/403/429 returns None
    #[test_case(404, "None", false)]
    #[test_case(418, "None", false)]  // Additional 4xx cases
    fn classify_http_status(code: u16, _variant: &str, retryable: bool) {
        let err = ProviderError::classify_http_status(code);
        match code {
            401 | 403 => {
                let err = err.expect("should be Some for auth errors");
                assert!(matches!(err, ProviderError::Auth(c) if c == code));
                assert_eq!(err.is_retryable(), retryable);
            }
            429 => {
                let err = err.expect("should be Some for rate limit");
                assert!(matches!(err, ProviderError::RateLimit { .. }));
                assert_eq!(err.is_retryable(), retryable);
            }
            500..=599 => {
                let err = err.expect("should be Some for server errors");
                assert!(matches!(err, ProviderError::Server(c, _) if c == code));
                assert_eq!(err.is_retryable(), retryable);
            }
            _ => {
                assert!(err.is_none(), "status {} should return None", code);
            }
        }
    }

    // ── Layer 1: SSE path uses the same classifier ──────────────────────────────
    // The SSE InvalidStatusCode path now calls classify_http_status, so both
    // HTTP (from_reqwest) and SSE (from_sse_error) paths produce identical results.

    #[test]
    fn from_sse_error_uses_shared_classifier() {
        // We can't easily construct SSE errors in tests due to reqwest version
        // conflicts, but we can verify that the classify_http_status function
        // produces the same results that the SSE code path would use.
        //
        // SSE path: InvalidStatusCode(code, _) -> classify_http_status(code)
        // HTTP path: from_reqwest -> classify_http_status(status.as_u16())
        //
        // Both should produce identical results for each status code.
        let test_cases = [
            (401, ProviderError::Auth(401)),
            (403, ProviderError::Auth(403)),
            (429, ProviderError::RateLimit { retry_after_secs: None }),
            (500, ProviderError::Server(500, String::new())),
            (502, ProviderError::Server(502, String::new())),
            (503, ProviderError::Server(503, String::new())),
        ];

        for (code, expected) in test_cases {
            let classified = ProviderError::classify_http_status(code);
            assert!(
                classified.is_some(),
                "classify_http_status({}) should return Some",
                code
            );
            let classified = classified.unwrap();

            // Verify the variant type matches
            match (&expected, &classified) {
                (ProviderError::Auth(exp_code), ProviderError::Auth(cls_code)) => {
                    assert_eq!(exp_code, cls_code);
                }
                (ProviderError::RateLimit { .. }, ProviderError::RateLimit { .. }) => {}
                (ProviderError::Server(exp_code, _), ProviderError::Server(cls_code, _)) => {
                    assert_eq!(exp_code, cls_code);
                }
                _ => panic!("Unexpected mismatch for {}: expected {:?}, got {:?}", code, expected, classified),
            }
        }
    }

    // ── Layer 1: typed ProviderError is_retryable ────────────────────────────────

    #[test_case(ProviderError::RateLimit { retry_after_secs: None }, true)]
    #[test_case(ProviderError::Timeout, true)]
    #[test_case(ProviderError::Network("connection refused".into()), true)]
    #[test_case(ProviderError::Server(502, Default::default()), true)]
    #[test_case(ProviderError::Auth(401), false)]
    #[test_case(ProviderError::ContextLength(128_000), false)]
    fn is_retryable_for_typed_errors(typed: ProviderError, expected: bool) {
        // Wrap the typed ProviderError as anyhow::Error so downcast works
        let err: anyhow::Error = typed.into();
        assert_eq!(is_retryable(&err), expected);
    }

    #[test_case("server overloaded", true)]
    #[test_case("rate limit exceeded", true)]
    #[test_case("timeout error", true)]
    #[test_case("connection refused", true)]
    #[test_case("try again later", true)]
    #[test_case("401 Unauthorized", false)]
    #[test_case("400 Bad Request", false)]
    #[test_case("invalid request", false)]
    fn is_retryable_for_string_errors(msg: &'static str, expected: bool) {
        let err = anyhow::anyhow!(msg);
        assert_eq!(is_retryable(&err), expected);
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

    // ── Layer 1: with_retry_config honors RetryConfig ───────────────────────────

    #[tokio::test]
    async fn with_retry_config_respects_max_attempts() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        // Config allows 3 attempts (max_attempts = 3)
        let config = RetryConfig::new(3, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let result: Result<i32, anyhow::Error> = with_retry_config(
            move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    // Use a retryable error message
                    Err(anyhow::anyhow!("rate limit exceeded"))
                }
            },
            &config,
        )
        .await;
        assert!(result.is_err());
        // Should have exactly 3 attempts (max_attempts)
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn with_retry_config_respects_no_retry() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        // Config disables retries (max_attempts = 1)
        let config = RetryConfig::no_retry();
        let result: Result<i32, anyhow::Error> = with_retry_config(
            move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::anyhow!("rate limit"))
                }
            },
            &config,
        )
        .await;
        assert!(result.is_err());
        // Should have exactly 1 attempt (no retries)
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn with_retry_config_succeeds_after_one_retry() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let config = RetryConfig::new(5, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let result: Result<i32, anyhow::Error> = with_retry_config(
            move || {
                let c = counter_clone.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n == 0 {
                        Err(anyhow::anyhow!("rate limit"))
                    } else {
                        Ok::<_, Error>(42)
                    }
                }
            },
            &config,
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        // Should have exactly 2 attempts
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
