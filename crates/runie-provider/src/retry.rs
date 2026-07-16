//! Retry configuration for provider streams.
//!
//! Uses `backon` for exponential backoff retry. The retry behavior is:
//! retry transient failures with exponential backoff, but only *before* the
//! stream starts emitting events. Once the stream has started, any error is
//! surfaced immediately.

use crate::{ProviderError, RetryConfig, RetryPolicy};
use anyhow::Error;
use futures::Future;
use std::time::Duration;
use tracing::Instrument;

/// Classifies error messages into typed `ProviderError` variants using string matching.
///
/// Patterns are matched case-insensitively. The classifier is stateless and cheap
/// to construct on each call — no caching needed since classification is O(n) on
/// the number of patterns.
#[derive(Debug, Clone, Default)]
pub struct ExceptionClassifier {
    _priv: (),
}

impl ExceptionClassifier {
    /// Classify an error into a typed `ProviderError` variant.
    ///
    /// Returns the most specific variant based on string matching:
    /// - `RateLimit` — HTTP 429, "rate limit", "service tier capacity", "too many requests"
    /// - `ContextLength` — "exceed context", "maximum context", "tokens exceeded",
    ///   "context window", "maximum tokens", "token limit"
    /// - `BadRequest` — content policy violations, responsible AI policy errors
    ///
    /// Returns `None` if the error doesn't match any known pattern.
    pub fn classify_error(&self, err: &Error) -> Option<ProviderError> {
        let msg = err.to_string();
        let msg_lower = msg.to_lowercase();

        // ── Rate limit patterns ──────────────────────────────────────────────────
        if self.matches_any(
            &msg_lower,
            &[
                "429",
                "rate limit",
                "too many requests",
                "service tier capacity",
                "request rate limit",
                "api rate limit",
            ],
        ) {
            return Some(ProviderError::RateLimit {
                retry_after_secs: None,
            });
        }

        // ── Context window / token limit patterns ───────────────────────────────
        if self.matches_any(
            &msg_lower,
            &[
                "exceed context",
                "maximum context",
                "context window",
                "tokens exceeded",
                "maximum tokens",
                "token limit",
                "context length",
                "too many tokens",
                "input too long",
                "maximum input length",
            ],
        ) {
            // Try to extract token count from the message
            let tokens = self.extract_token_count(&msg);
            return Some(ProviderError::ContextLength(tokens));
        }

        // ── Content policy / responsible AI patterns ─────────────────────────────
        if self.matches_any(
            &msg_lower,
            &[
                "content_policy_violation",
                "responsible_ai_policy",
                "content policy",
                "harmful content",
                "safety policy",
                "moderation",
                "inappropriate content",
            ],
        ) {
            return Some(ProviderError::BadRequest(400, msg));
        }

        // ── Auth patterns (non-retryable) ────────────────────────────────────────
        if self.matches_any(
            &msg_lower,
            &["401", "403", "unauthorized", "forbidden", "invalid api key"],
        ) {
            // Extract status code if present; otherwise infer from keywords.
            let code = self.extract_status_code(&msg).unwrap_or_else(|| {
                if msg_lower.contains("unauthorized") {
                    401
                } else {
                    403
                }
            });
            return Some(ProviderError::Auth(code));
        }

        // ── Generic client errors (non-retryable) ────────────────────────────────
        if self.matches_any(
            &msg_lower,
            &["400", "404", "402", "422", "bad request", "not found"],
        ) {
            let code = self.extract_status_code(&msg).unwrap_or(400);
            return Some(ProviderError::BadRequest(code, msg));
        }

        // ── Server errors (retryable) ────────────────────────────────────────────
        if self.matches_any(
            &msg_lower,
            &["500", "502", "503", "504", "server error", "internal error"],
        ) {
            let code = self.extract_status_code(&msg).unwrap_or(500);
            return Some(ProviderError::Server(code, msg));
        }

        None
    }

    /// Returns true if `msg` contains any of the `patterns` (case-insensitive).
    fn matches_any(&self, msg: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|p| msg.contains(p))
    }

    /// Extract token count from a context window error message if present.
    /// Looks for common numeric patterns like "128000 tokens" or "context window of 200000".
    fn extract_token_count(&self, msg: &str) -> usize {
        let msg_lower = msg.to_lowercase();
        // Simple regex-free heuristic: find the first 4-7 digit number that is
        // near a token/context keyword. The keyword may appear before or after
        // the number, so scan a window around it.
        let keywords = ["tokens", "token", "context", "max_tokens", "maximum tokens"];
        let mut digits = String::new();
        let mut digit_start = 0usize;
        for (i, c) in msg_lower.char_indices() {
            if c.is_ascii_digit() {
                if digits.is_empty() {
                    digit_start = i;
                }
                digits.push(c);
            } else {
                if (4..=7).contains(&digits.len()) {
                    let window_start = digit_start.saturating_sub(30);
                    let window_end = (i + 30).min(msg_lower.len());
                    let window = &msg_lower[window_start..window_end];
                    if keywords.iter().any(|k| window.contains(k)) {
                        if let Ok(n) = digits.parse::<usize>() {
                            return n;
                        }
                    }
                }
                digits.clear();
            }
        }
        // Trailing digits at end of string.
        if (4..=7).contains(&digits.len()) {
            let window_start = digit_start.saturating_sub(30);
            let window_end = msg_lower.len();
            let window = &msg_lower[window_start..window_end];
            if keywords.iter().any(|k| window.contains(k)) {
                if let Ok(n) = digits.parse::<usize>() {
                    return n;
                }
            }
        }
        0
    }

    /// Extract HTTP status code from an error message if present.
    fn extract_status_code(&self, msg: &str) -> Option<u16> {
        // Match patterns like "HTTP 429", "(429)", "status 429", "429"
        let mut digits = String::new();
        for c in msg.chars() {
            if c.is_ascii_digit() {
                digits.push(c);
                if digits.len() == 3 {
                    return digits.parse::<u16>().ok();
                }
            } else {
                digits.clear();
            }
        }
        None
    }
}

impl Default for &ExceptionClassifier {
    fn default() -> Self {
        static DEFAULT: ExceptionClassifier = ExceptionClassifier { _priv: () };
        &DEFAULT
    }
}

/// Classify an SSE stream error into a typed `ProviderError` variant.
pub fn from_sse_error(err: &reqwest_eventsource::Error) -> ProviderError {
    use reqwest_eventsource::Error as SseErr;
    match err {
        // UTF-8 decode error — wrap as source
        SseErr::Utf8(_) => ProviderError::Source(anyhow::anyhow!("{err}")),
        // Parser error — wrap as source
        SseErr::Parser(_) => ProviderError::Source(anyhow::anyhow!("{err}")),
        // HTTP-level error from reqwest
        SseErr::Transport(e) => ProviderError::from_reqwest(e),
        // Content-type mismatch
        SseErr::InvalidContentType(_, _) => ProviderError::Source(anyhow::anyhow!("{err}")),
        // HTTP status code error (5xx, 429, 401, 403) — use shared classifier
        SseErr::InvalidStatusCode(status, _) => {
            let code = status.as_u16();
            ProviderError::classify_http_status(code)
                .unwrap_or_else(|| ProviderError::Source(anyhow::anyhow!("{err}")))
        }
        // Invalid Last-Event-ID header
        SseErr::InvalidLastEventId(_) => ProviderError::Source(anyhow::anyhow!("{err}")),
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
    // Fallback: use ExceptionClassifier for string-based classification
    let classifier = ExceptionClassifier::default();
    if let Some(typed) = classifier.classify_error(e) {
        return typed.is_retryable();
    }
    // Last resort: legacy heuristics for errors not caught by classifier
    let msg = e.to_string().to_lowercase();
    msg.contains("timeout")
        || msg.contains("connection")
        || msg.contains("overloaded")
        || msg.contains("try again")
}

/// Retry a fallible async operation with exponential backoff using `backon`.
///
/// Uses default retry parameters. For custom retry behavior, use
/// [`with_retry_config`](with_retry_config) or [`with_retry_policy`](with_retry_policy).
pub async fn with_retry<F, Fut, T>(f: F) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    with_retry_policy(f, &RetryPolicy::default()).await
}

/// Retry a fallible async operation with custom retry configuration.
///
/// This function converts `RetryConfig` to a `RetryPolicy` and delegates to
/// [`with_retry_policy`](with_retry_policy). For per-error-type retry counts,
/// use [`with_retry_policy`](with_retry_policy) directly.
pub async fn with_retry_config<F, Fut, T>(f: F, config: &RetryConfig) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    with_retry_policy(f, &config.clone().into_policy()).await
}

/// Retry a fallible async operation with per-error-type retry policy.
///
/// Converts `RetryPolicy` to backon's `ExponentialBuilder`:
/// - `base.max_attempts` → `with_max_times()` (backon counts total attempts)
/// - `base.initial_delay` → `with_min_delay()`
/// - `base.max_delay` → `with_max_delay()`
/// - `base.multiplier` → `with_factor()`
///
/// For typed errors, uses the per-error-type retry count if configured:
/// - `rate_limit_retries` for `RateLimit` errors
/// - `timeout_retries` for `Timeout` errors
/// - `context_window_retries` for `ContextLength` errors (fatal by default)
/// - `bad_request_retries` for `BadRequest` errors (fatal by default)
pub async fn with_retry_policy<F, Fut, T>(f: F, policy: &RetryPolicy) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let base = &policy.base;

    // Determine max attempts from policy (this will be overridden per error type in the retry loop)
    let max_attempts = base.max_attempts;
    let span = tracing::info_span!("provider_retry", max_attempts = %max_attempts);

    let result = async {
        let mut attempts = 0u32;

        loop {
            match f().await {
                Ok(result) => {
                    if attempts > 1 {
                        tracing::debug!(attempts = %attempts, "provider retry succeeded after retries");
                    } else {
                        tracing::debug!("provider retry succeeded on first attempt");
                    }
                    return Ok(result);
                }
                Err(e) => {
                    attempts += 1;

                    // Classify the error so we can apply the per-error-type policy.
                    let typed_error = if let Some(typed) = e.downcast_ref::<ProviderError>() {
                        typed.clone()
                    } else if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
                        ProviderError::from_reqwest(reqwest_err)
                    } else {
                        let classifier = ExceptionClassifier::default();
                        classifier
                            .classify_error(&e)
                            .unwrap_or_else(|| ProviderError::Source(anyhow::anyhow!("{e}")))
                    };

                    let error_max_attempts = policy.max_attempts_for_error(&typed_error);

                    // Retry if the error is naturally retryable or if the policy has an
                    // explicit override that allows more attempts than the base config.
                    let retryable =
                        typed_error.is_retryable() || error_max_attempts > base.max_attempts;

                    if !retryable {
                        tracing::warn!(error = %e, attempts = %attempts, "provider retry: error is not retryable");
                        return Err(e);
                    }

                    // Check if we've exceeded this error type's retry count
                    if attempts >= error_max_attempts {
                        tracing::warn!(
                            error = %e,
                            attempts = %attempts,
                            error_max_attempts = %error_max_attempts,
                            "provider retry: exceeded retry count for this error type"
                        );
                        return Err(e);
                    }

                    // Calculate delay with exponential backoff
                    let delay = calculate_delay(attempts, base.initial_delay, base.max_delay, base.multiplier);
                    tracing::debug!(
                        error = %e,
                        attempts = %attempts,
                        next_delay_ms = %delay.as_millis(),
                        "provider retry: sleeping before next attempt"
                    );

                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    .instrument(span)
    .await;

    result
}

/// Calculate exponential backoff delay.
fn calculate_delay(
    attempt: u32,
    initial_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
) -> Duration {
    let exp_delay = (initial_delay.as_millis() as f64) * multiplier.powi(attempt as i32 - 1);
    let delayed = Duration::from_millis(exp_delay as u64);
    delayed.min(max_delay)
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
    #[test_case(408, "Server(408)", true)]
    #[test_case(429, "RateLimit", true)]
    #[test_case(500, "Server(500)", true)]
    #[test_case(502, "Server(502)", true)]
    #[test_case(503, "Server(503)", true)]
    #[test_case(529, "Server(529)", true)]
    #[test_case(400, "BadRequest", false)]
    #[test_case(404, "BadRequest", false)]
    #[test_case(418, "BadRequest", false)] // Additional 4xx cases
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
            408 | 500..=599 => {
                let err = err.expect("should be Some for retryable server/status errors");
                assert!(matches!(err, ProviderError::Server(c, _) if c == code));
                assert_eq!(err.is_retryable(), retryable);
            }
            400..=499 => {
                let err = err.expect("should be Some for client errors");
                assert!(matches!(err, ProviderError::BadRequest(c, _) if c == code));
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
            (
                429,
                ProviderError::RateLimit {
                    retry_after_secs: None,
                },
            ),
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
                _ => panic!(
                    "Unexpected mismatch for {}: expected {:?}, got {:?}",
                    code, expected, classified
                ),
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
    #[test_case(ProviderError::BadRequest(400, "invalid_request".into()), false)]
    #[test_case(ProviderError::BadRequest(404, "not found".into()), false)]
    #[test_case(ProviderError::Server(529, "overloaded".into()), true)]
    fn is_retryable_for_typed_errors(typed: ProviderError, expected: bool) {
        // Wrap the typed ProviderError as anyhow::Error so downcast works
        let err: anyhow::Error = typed.into();
        assert_eq!(is_retryable(&err), expected);
    }

    #[test]
    fn is_retryable_classifies_400_429_529() {
        let bad = ProviderError::classify_http_status(400).expect("400 should classify");
        assert!(!bad.is_retryable(), "400 must not be retried");

        let rate = ProviderError::classify_http_status(429).expect("429 should classify");
        assert!(rate.is_retryable(), "429 must be retried");

        let overloaded = ProviderError::classify_http_status(529).expect("529 should classify");
        assert!(overloaded.is_retryable(), "529 must be retried");
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

    // ── Per-error-type retry policy tests ─────────────────────────────────────────

    #[tokio::test]
    async fn with_retry_policy_rate_limit_retries() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        // Rate limit errors get 5 retries (10 total attempts)
        let base = RetryConfig::new(3, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new(base, Some(10), None, None, None);

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n < 9 {
                        // Return a rate limit error
                        Err(ProviderError::RateLimit {
                            retry_after_secs: None,
                        }
                        .into())
                    } else {
                        Ok(42)
                    }
                }
            },
            &policy,
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        // Should have exactly 10 attempts (1 initial + 9 retries)
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn with_retry_policy_timeout_retries() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        // Timeout errors get 4 retries (5 total attempts)
        let base = RetryConfig::new(2, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new(base, None, Some(5), None, None);

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n < 4 {
                        // Return a timeout error
                        Err(ProviderError::Timeout.into())
                    } else {
                        Ok(42)
                    }
                }
            },
            &policy,
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        // Should have exactly 5 attempts
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn with_retry_policy_context_window_fatal_by_default() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        // Context length errors are fatal (non-retryable) by default
        let base = RetryConfig::new(10, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new(base, None, None, None, None);

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    // Return a context length error
                    Err(ProviderError::ContextLength(128000).into())
                }
            },
            &policy,
        )
        .await;
        assert!(result.is_err());
        // Should have exactly 1 attempt (fatal error, no retries)
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn with_retry_policy_context_window_with_retries_enabled() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        // With context_window_retries set, context length errors become retryable
        let base = RetryConfig::new(2, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new(base, None, None, Some(5), None);

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n < 4 {
                        // Return a context length error
                        Err(ProviderError::ContextLength(128000).into())
                    } else {
                        Ok(42)
                    }
                }
            },
            &policy,
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        // Should have exactly 5 attempts
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn with_retry_policy_bad_request_fatal_by_default() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        // Bad request errors are fatal (non-retryable) by default
        let base = RetryConfig::new(10, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new(base, None, None, None, None);

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    // Return a bad request error
                    Err(ProviderError::BadRequest(400, "invalid request".to_string()).into())
                }
            },
            &policy,
        )
        .await;
        assert!(result.is_err());
        // Should have exactly 1 attempt (fatal error, no retries)
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn with_retry_policy_bad_request_with_retries_enabled() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        // With bad_request_retries set, bad request errors become retryable
        let base = RetryConfig::new(2, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new(base, None, None, None, Some(4));

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n < 3 {
                        // Return a bad request error
                        Err(ProviderError::BadRequest(400, "invalid request".to_string()).into())
                    } else {
                        Ok(42)
                    }
                }
            },
            &policy,
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        // Should have exactly 4 attempts
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn with_retry_policy_server_error_uses_base_config() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        // Server errors use base config (no per-error-type override)
        let base = RetryConfig::new(4, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new(base, Some(10), Some(10), Some(10), Some(10));

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n < 3 {
                        // Return a server error (uses base config)
                        Err(ProviderError::Server(500, "internal error".to_string()).into())
                    } else {
                        Ok(42)
                    }
                }
            },
            &policy,
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        // Should use base config: 4 attempts
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    // ── ExceptionClassifier tests ─────────────────────────────────────────────────

    #[test]
    fn classifier_matches_rate_limit_patterns() {
        let classifier = ExceptionClassifier::default();
        let cases = [
            "HTTP 429 Rate limit exceeded",
            "rate limit exceeded",
            "Too Many Requests",
            "Service tier capacity limit reached",
            "API rate limit exceeded for this endpoint",
            "request rate limit exceeded",
        ];
        for msg in cases {
            let err = anyhow::anyhow!(msg);
            let classified = classifier.classify_error(&err);
            assert!(
                matches!(classified, Some(ProviderError::RateLimit { .. })),
                "Expected RateLimit for: {msg}"
            );
        }
    }

    #[test]
    fn classifier_matches_context_window_patterns() {
        let classifier = ExceptionClassifier::default();
        let cases = [
            ("exceeded context window: 128000 tokens", 128_000),
            ("maximum context length exceeded", 0), // default when count not extracted
            ("maximum context: 200000 tokens", 200_000),
            ("context window of 32000 tokens", 32_000),
            ("tokens exceeded limit of 100000", 100_000),
            ("maximum tokens per request exceeded", 0),
            ("input too long: exceeds 8192 tokens", 8192),
            ("context length: 65536 tokens", 65_536),
        ];
        for (msg, expected_tokens) in cases {
            let err = anyhow::anyhow!(msg);
            let classified = classifier.classify_error(&err);
            assert!(
                matches!(classified, Some(ProviderError::ContextLength(n)) if n == expected_tokens),
                "Expected ContextLength({expected_tokens}) for: {msg}, got: {classified:?}"
            );
        }
    }

    #[test]
    fn classifier_matches_content_policy_patterns() {
        let classifier = ExceptionClassifier::default();
        let cases = [
            "content_policy_violation: request blocked",
            "Content policy violation detected",
            "RESPONSIBLE_AI_POLICY: harmful content blocked",
            "content policy violation: sensitive topic",
            "Safety policy violation: inappropriate request",
            "Harmful content blocked by moderation system",
            "inappropriate content request rejected",
        ];
        for msg in cases {
            let err = anyhow::anyhow!(msg);
            let classified = classifier.classify_error(&err);
            assert!(
                matches!(classified, Some(ProviderError::BadRequest(400, _))),
                "Expected BadRequest(400) for: {msg}"
            );
        }
    }

    #[test]
    fn classifier_matches_auth_patterns() {
        let classifier = ExceptionClassifier::default();
        let cases = [
            ("HTTP 401 Unauthorized", 401),
            ("403 Forbidden: invalid credentials", 403),
            ("Request forbidden", 403),
            ("unauthorized: missing API key", 401),
            ("Invalid API key provided", 403),
        ];
        for (msg, expected_code) in cases {
            let err = anyhow::anyhow!(msg);
            let classified = classifier.classify_error(&err);
            assert!(
                matches!(classified, Some(ProviderError::Auth(code)) if code == expected_code),
                "Expected Auth({expected_code}) for: {msg}"
            );
        }
    }

    #[test]
    fn classifier_matches_server_error_patterns() {
        let classifier = ExceptionClassifier::default();
        let cases = [
            ("HTTP 500 Internal Server Error", 500),
            ("502 Bad Gateway", 502),
            ("503 Service Temporarily Unavailable", 503),
            ("504 Gateway Timeout", 504),
            ("Internal server error occurred", 500),
        ];
        for (msg, expected_code) in cases {
            let err = anyhow::anyhow!(msg);
            let classified = classifier.classify_error(&err);
            assert!(
                matches!(classified, Some(ProviderError::Server(code, _)) if code == expected_code),
                "Expected Server({expected_code}) for: {msg}"
            );
        }
    }

    #[test]
    fn classifier_matches_client_error_patterns() {
        let classifier = ExceptionClassifier::default();
        let cases = [
            ("HTTP 400 Bad Request", 400),
            ("404 Not Found: resource does not exist", 404),
            ("402 Payment Required", 402),
            ("422 Unprocessable Entity", 422),
            ("Bad request: invalid parameter", 400),
        ];
        for (msg, expected_code) in cases {
            let err = anyhow::anyhow!(msg);
            let classified = classifier.classify_error(&err);
            assert!(
                matches!(classified, Some(ProviderError::BadRequest(code, _)) if code == expected_code),
                "Expected BadRequest({expected_code}) for: {msg}"
            );
        }
    }

    #[test]
    fn classifier_returns_none_for_unknown_errors() {
        let classifier = ExceptionClassifier::default();
        let cases = [
            "some cryptic error message",
            "unexpected failure",
            "operation failed for unknown reason",
        ];
        for msg in cases {
            let err = anyhow::anyhow!(msg);
            let classified = classifier.classify_error(&err);
            assert!(
                classified.is_none(),
                "Expected None for: {msg}, got: {classified:?}"
            );
        }
    }

    #[test]
    fn is_retryable_uses_classifier_for_unknown_errors() {
        // Rate limit via classifier → retryable
        let err = anyhow::anyhow!("Rate limit exceeded for API calls");
        assert!(is_retryable(&err), "Rate limit should be retryable");

        // Context window via classifier → not retryable
        let err = anyhow::anyhow!("Context window of 128000 tokens exceeded");
        assert!(
            !is_retryable(&err),
            "Context length should not be retryable"
        );

        // Content policy via classifier → not retryable
        let err = anyhow::anyhow!("content_policy_violation: request blocked");
        assert!(
            !is_retryable(&err),
            "Content policy should not be retryable"
        );

        // Auth via classifier → not retryable
        let err = anyhow::anyhow!("401 Unauthorized: invalid credentials");
        assert!(!is_retryable(&err), "Auth errors should not be retryable");

        // Server error via classifier → retryable
        let err = anyhow::anyhow!("HTTP 503 Service Unavailable");
        assert!(is_retryable(&err), "Server errors should be retryable");
    }

    #[test]
    fn is_retryable_legacy_heuristics_still_work() {
        // Legacy heuristics (no longer covered by classifier)
        let err = anyhow::anyhow!("connection refused");
        assert!(is_retryable(&err), "Connection errors should be retryable");

        let err = anyhow::anyhow!("server overloaded");
        assert!(is_retryable(&err), "Overloaded should be retryable");

        let err = anyhow::anyhow!("try again later");
        assert!(is_retryable(&err), "Try again should be retryable");
    }
}
