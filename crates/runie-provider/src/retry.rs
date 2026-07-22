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
use std::collections::HashSet;
use tracing::Instrument;

/// Classifies string-based error messages into typed `ProviderError` variants.
///
/// This classifier handles error messages from various LLM providers that don't
/// map cleanly to HTTP status codes. It covers:
///
/// - **Rate limits**: 429 responses, "rate limit", "service tier", "quota exceeded"
/// - **Context windows**: "exceed context", "maximum context", "tokens exceeded", "input too long"
/// - **Content policy**: "content policy", "safety", "harmful content", "blocked"
/// - **Network/timeout**: "timeout", "connection", "unreachable"
/// - **Server errors**: "overloaded", "internal error", "503"
/// - **Auth errors**: "401", "403", "unauthorized", "invalid api key"
pub struct ExceptionClassifier {
    // Pre-compiled patterns for fast lookup
    rate_limit_patterns: HashSet<&'static str>,
    context_window_patterns: HashSet<&'static str>,
    content_policy_patterns: HashSet<&'static str>,
    network_patterns: HashSet<&'static str>,
    transient_patterns: HashSet<&'static str>,
    fatal_patterns: HashSet<&'static str>,
}

impl Default for ExceptionClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ExceptionClassifier {
    /// Create a new classifier with comprehensive error patterns.
    #[allow(clippy::too_many_lines)]
    pub fn new() -> Self {
        Self {
            // Rate limiting patterns
            rate_limit_patterns: [
                // Explicit rate limit indicators
                "rate limit",
                "rate_limit",
                "rate-limit",
                "too many requests",
                "429",
                // Quota and tier limits
                "quota exceeded",
                "quota limit",
                "monthly quota",
                "daily quota",
                "service tier",
                "tier limit",
                "limit exceeded",
                // Retry indicators
                "try again",
                "retry after",
                "backoff",
                "slow down",
            ]
            .into(),
            // Context window patterns
            context_window_patterns: [
                // Context length exceeded
                "exceed context",
                "context window",
                "maximum context",
                "context length",
                "context limit",
                "too many tokens",
                "tokens exceeded",
                "token limit",
                "input too long",
                "message too long",
                "max tokens",
                "context size",
                "too long for model",
                "exceeds maximum",
                // Specific provider messages
                "maximum input",
                "input length",
                "output length",
                "total length",
            ]
            .into(),
            // Content policy violation patterns
            content_policy_patterns: [
                // Generic policy violations
                "content policy",
                "policy violation",
                "policy_error",
                "violates policy",
                // Safety-related
                "safety",
                "safety filter",
                "harmful content",
                "harmful request",
                "inappropriate content",
                "restricted content",
                "blocked content",
                // Specific provider terms
                "cannot assist",
                "unable to assist",
                "cannot provide",
                "blocked by policy",
                "harmfulness",
                "moderation",
                "flagged content",
                // Harm categories
                "hate speech",
                "violence",
                "self-harm",
                "sexual content",
            ]
            .into(),
            // Network/connectivity patterns
            network_patterns: [
                // Connection issues
                "connection",
                "connect failed",
                "connection refused",
                "connection reset",
                "connection timed out",
                "network error",
                "network failure",
                "unreachable",
                "host not found",
                "dns",
                // General network
                "socket",
                "econnrefused",
                "etimedout",
                "enetunreach",
            ]
            .into(),
            // Transient server errors (retryable)
            transient_patterns: [
                // Server overload
                "overloaded",
                "server overloaded",
                "service unavailable",
                "503",
                "500",
                "502",
                "504",
                "bad gateway",
                "gateway timeout",
                "internal server error",
                // Temporary issues
                "temporary failure",
                "temporary error",
                "service disruption",
                "maintenance",
            ]
            .into(),
            // Fatal error patterns (should not retry)
            fatal_patterns: [
                // Authentication
                "401",
                "403",
                "unauthorized",
                "forbidden",
                "invalid api key",
                "invalid api_key",
                "api key",
                "authentication",
                "auth failed",
                "auth error",
                // Bad request patterns (non-context-window related)
                "400 bad request",
                "invalid request",
                "malformed request",
                // Model not found
                "model not found",
                "model does not exist",
                "unknown model",
            ]
            .into(),
        }
    }

    /// Classify a string error message into a typed `ProviderError` variant.
    ///
    /// Patterns are checked in order of priority:
    /// 1. Context window errors → `ProviderError::ContextLength` (fatal)
    /// 2. Content policy violations → `ProviderError::Source` (fatal)
    /// 3. Rate limit errors → `ProviderError::RateLimit` (retryable)
    /// 4. Network errors → `ProviderError::Network` (retryable)
    /// 5. Auth errors → `ProviderError::Auth` (fatal)
    /// 6. Timeout → `ProviderError::Timeout` (retryable)
    /// 7. Server/transient errors → `ProviderError::Server` (retryable)
    /// 8. No match → `ProviderError::Source` (retryable, conservative)
    pub fn classify(&self, error_msg: &str) -> ProviderError {
        let msg = error_msg.to_lowercase();

        // Priority 1: Context window errors (fatal - don't retry with same prompt)
        if self.matches_any(&msg, &self.context_window_patterns) {
            // Try to extract token count if present
            let tokens = self.extract_token_count(&msg);
            return ProviderError::ContextLength(tokens);
        }

        // Priority 2: Content policy violations (fatal - won't be fixed by retry)
        if self.matches_any(&msg, &self.content_policy_patterns) {
            return ProviderError::Source(anyhow::anyhow!("Content policy violation: {}", error_msg));
        }

        // Priority 3: Rate limit errors (retryable)
        if self.matches_any(&msg, &self.rate_limit_patterns) {
            let retry_after = self.extract_retry_after(&msg);
            return ProviderError::RateLimit { retry_after_secs: retry_after };
        }

        // Priority 4: Network errors (retryable)
        if self.matches_any(&msg, &self.network_patterns) {
            return ProviderError::Network(error_msg.to_string());
        }

        // Priority 5: Timeout errors (retryable)
        {
            let timeout_patterns: HashSet<_> = ["timeout", "timed out", "timedout"].into();
            if self.matches_any(&msg, &timeout_patterns) {
                return ProviderError::Timeout;
            }
        }

        // Priority 6: Auth errors (fatal)
        if self.matches_any(&msg, &self.fatal_patterns) {
            let code = self.extract_http_code(&msg).unwrap_or(0);
            return ProviderError::Auth(code);
        }

        // Priority 7: Server/transient errors (retryable)
        if self.matches_any(&msg, &self.transient_patterns) {
            let code = self.extract_http_code(&msg).unwrap_or(500);
            return ProviderError::Server(code, error_msg.to_string());
        }

        // Fallback: wrap in Source (retryable by default)
        ProviderError::Source(anyhow::anyhow!("{}", error_msg))
    }

    /// Check if the message matches any of the given patterns.
    fn matches_any(&self, msg: &str, patterns: &HashSet<&'static str>) -> bool {
        for pattern in patterns {
            if msg.contains(pattern) {
                return true;
            }
        }
        false
    }

    /// Extract HTTP status code from error message if present.
    fn extract_http_code(&self, msg: &str) -> Option<u16> {
        // Match common HTTP error formats: "401", "403", "429", "500", etc.
        let codes = ["401", "403", "408", "429", "500", "502", "503", "504"];
        for code in codes {
            if msg.contains(code) {
                return code.parse().ok();
            }
        }
        None
    }

    /// Extract retry-after seconds from error message if present.
    fn extract_retry_after(&self, msg: &str) -> Option<u32> {
        // Look for "retry after X seconds" or "retry_after: X"
        if let Some(pos) = msg.find("retry after") {
            let after = &msg[pos..];
            // Try to extract number
            if let Ok(num) = after
                .chars()
                .skip(11) // Skip "retry after "
                .take(10)
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
            {
                return Some(num);
            }
        }
        if let Some(pos) = msg.find("retry_after") {
            let after = &msg[pos..];
            if let Ok(num) = after
                .chars()
                .skip(12) // Skip "retry_after: "
                .take(10)
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
            {
                return Some(num);
            }
        }
        None
    }

    /// Extract token count from error message if present.
    fn extract_token_count(&self, msg: &str) -> usize {
        // Look for patterns like "128000 tokens", "exceeds 200000 tokens", etc.
        if let Some(pos) = msg.find(|c| char::is_ascii_digit(&c)) {
            let num_str: String = msg[pos..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(n) = num_str.parse() {
                return n;
            }
        }
        // Default to a large number if we can't extract
        0
    }
}

/// Check if the message matches any of the given patterns (standalone helper).
#[allow(dead_code)]
fn matches_pattern(msg: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| msg.contains(p))
}

/// Per-error-type retry policy for fine-grained retry control.
///
/// Each field specifies the maximum number of retries for that error category.
/// Setting a field to `None` uses the global `max_attempts` from `RetryConfig`.
/// Setting a field to `Some(0)` disables retries for that error type.
#[derive(Clone, Debug, Default)]
pub struct RetryPolicy {
    /// Maximum retries for rate limit errors (429).
    pub rate_limit_retries: Option<u32>,
    /// Maximum retries for timeout errors.
    pub timeout_retries: Option<u32>,
    /// Maximum retries for context window exceeded errors (fatal by default).
    pub context_window_retries: Option<u32>,
    /// Maximum retries for bad request / auth errors (fatal by default).
    pub bad_request_retries: Option<u32>,
}

impl RetryPolicy {
    /// Create a new retry policy with default values (all None = use global config).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a policy that disables all per-error-type retries (use global config only).
    pub fn use_global_config() -> Self {
        Self::default()
    }

    /// Set the maximum retries for rate limit errors.
    pub fn with_rate_limit_retries(mut self, retries: Option<u32>) -> Self {
        self.rate_limit_retries = retries;
        self
    }

    /// Set the maximum retries for timeout errors.
    pub fn with_timeout_retries(mut self, retries: Option<u32>) -> Self {
        self.timeout_retries = retries;
        self
    }

    /// Set the maximum retries for context window exceeded errors.
    pub fn with_context_window_retries(mut self, retries: Option<u32>) -> Self {
        self.context_window_retries = retries;
        self
    }

    /// Set the maximum retries for bad request / auth errors.
    pub fn with_bad_request_retries(mut self, retries: Option<u32>) -> Self {
        self.bad_request_retries = retries;
        self
    }

    /// Get the maximum retries for a given error based on its type.
    /// Returns `None` if this error should not be retried at all.
    /// Returns `Some(n)` with the number of additional retry attempts allowed.
    pub fn max_retries_for(&self, err: &Error) -> Option<u32> {
        if !is_retryable(err) {
            return None;
        }

        // Try to get typed ProviderError first
        if let Some(typed) = err.downcast_ref::<ProviderError>() {
            return self.max_retries_for_typed(typed);
        }

        // Fallback to string heuristics - use global config (None = no override)
        Some(u32::MAX)
    }

    fn max_retries_for_typed(&self, err: &ProviderError) -> Option<u32> {
        match err {
            ProviderError::RateLimit { .. } => self.rate_limit_retries,
            ProviderError::Timeout => self.timeout_retries,
            ProviderError::ContextLength(_) => self.context_window_retries,
            // Auth errors are typically not retryable by default
            // but we allow configuration for cases like token refresh
            ProviderError::Auth(_) => self.bad_request_retries,
            // For server errors, network errors, and unknown errors,
            // use the global config (return None to let backon decide)
            _ => None,
        }
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
            ProviderError::classify_http_status(code).unwrap_or_else(|| ProviderError::Source(anyhow::anyhow!("{err}")))
        }
        // Invalid Last-Event-ID header
        SseErr::InvalidLastEventId(_) => ProviderError::Source(anyhow::anyhow!("{err}")),
        // Stream ended unexpectedly
        SseErr::StreamEnded => ProviderError::Source(anyhow::anyhow!("SSE stream ended unexpectedly")),
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
    // Fallback: use ExceptionClassifier for comprehensive string-based classification
    let classifier = ExceptionClassifier::new();
    let classified = classifier.classify(&e.to_string());
    classified.is_retryable()
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
pub async fn with_retry_config<F, Fut, T>(f: F, config: &RetryConfig) -> Result<T, Error>
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

/// Retry a fallible async operation with per-error-type retry policy.
///
/// This function extends `with_retry_config` by allowing different retry counts
/// for different error types. Each error type can have its own retry limit,
/// overriding the global `max_attempts` from `RetryConfig`.
///
/// # Arguments
/// * `f` - The async operation to retry
/// * `config` - Base retry configuration (timing parameters)
/// * `policy` - Per-error-type retry limits
///
/// # Example
/// ```ignore
/// let policy = RetryPolicy::new()
///     .with_rate_limit_retries(Some(10))  // More retries for rate limits
///     .with_timeout_retries(Some(5))       // Fewer retries for timeouts
///     .with_context_window_retries(Some(0)); // No retries for context errors
///
/// with_retry_policy(operation, &config, &policy).await;
/// ```
pub async fn with_retry_policy<F, Fut, T>(f: F, config: &RetryConfig, policy: &RetryPolicy) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    with_retry_policy_internal(f, config, policy, is_retryable).await
}

/// Internal implementation of retry with per-error-type policy.
///
/// Uses a custom retry condition that checks both:
/// 1. Whether the error is retryable at all (`is_retryable`)
/// 2. Whether we've exceeded the per-error-type retry limit
#[allow(clippy::too_many_lines)]
async fn with_retry_policy_internal<F, Fut, T, FIsRetryable>(
    f: F,
    config: &RetryConfig,
    policy: &RetryPolicy,
    is_retryable_fn: FIsRetryable,
) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
    FIsRetryable: Fn(&Error) -> bool + Copy,
{
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    // Track retry counts per error category
    let rate_limit_retries = Arc::new(AtomicU32::new(0));
    let timeout_retries = Arc::new(AtomicU32::new(0));
    let context_window_retries = Arc::new(AtomicU32::new(0));
    let bad_request_retries = Arc::new(AtomicU32::new(0));

    let builder = ExponentialBuilder::default()
        .with_max_times(config.max_attempts.saturating_sub(1) as usize)
        .with_min_delay(config.initial_delay)
        .with_max_delay(config.max_delay)
        .with_factor(config.multiplier as f32);

    tracing::debug!(
        max_attempts = %config.max_attempts,
        initial_delay_ms = %config.initial_delay.as_millis(),
        max_delay_ms = %config.max_delay.as_millis(),
        rate_limit_retries = ?policy.rate_limit_retries,
        timeout_retries = ?policy.timeout_retries,
        context_window_retries = ?policy.context_window_retries,
        bad_request_retries = ?policy.bad_request_retries,
        "provider retry with per-error-type policy starting"
    );

    let span = tracing::info_span!(
        "provider_retry_policy",
        max_attempts = %config.max_attempts
    );

    let result = async {
        let policy = policy.clone();
        let rate_limit_retries = rate_limit_retries.clone();
        let timeout_retries = timeout_retries.clone();
        let context_window_retries = context_window_retries.clone();
        let bad_request_retries = bad_request_retries.clone();

        // Custom retry condition that checks per-error-type limits
        let retry_condition = move |err: &Error| -> bool {
            // First check if error is retryable at all
            if !is_retryable_fn(err) {
                return false;
            }

            // Try to classify the error
            let (category, counter, max_retries) = if let Some(typed) = err.downcast_ref::<ProviderError>() {
                match typed {
                    ProviderError::RateLimit { .. } => (
                        "rate_limit",
                        &rate_limit_retries,
                        &policy.rate_limit_retries,
                    ),
                    ProviderError::Timeout => ("timeout", &timeout_retries, &policy.timeout_retries),
                    ProviderError::ContextLength(_) => (
                        "context_window",
                        &context_window_retries,
                        &policy.context_window_retries,
                    ),
                    ProviderError::Auth(_) => (
                        "bad_request",
                        &bad_request_retries,
                        &policy.bad_request_retries,
                    ),
                    _ => return true, // Use global config for other error types
                }
            } else {
                // Unknown error type - use global config
                return true;
            };

            // Check if this category has a specific retry limit
            if let Some(max) = max_retries {
                let current = counter.load(Ordering::SeqCst);
                if current >= *max {
                    tracing::debug!(
                        error_category = %category,
                        current_retries = %current,
                        max_retries = %max,
                        "per-error-type retry limit reached"
                    );
                    return false;
                }
                counter.fetch_add(1, Ordering::SeqCst);
            }

            true
        };

        f.retry(builder)
            .when(retry_condition)
            .await
            .inspect_err(|e| tracing::warn!(error = %e, "provider retry failed"))
    }
    .instrument(span)
    .await;

    if result.is_ok() {
        tracing::debug!(
            rate_limit_retries = %rate_limit_retries.load(Ordering::SeqCst),
            timeout_retries = %timeout_retries.load(Ordering::SeqCst),
            context_window_retries = %context_window_retries.load(Ordering::SeqCst),
            bad_request_retries = %bad_request_retries.load(Ordering::SeqCst),
            "provider retry with policy succeeded"
        );
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
    #[test_case(400, "None", false)] // 4xx other than 401/403/429 returns None
    #[test_case(404, "None", false)]
    #[test_case(418, "None", false)] // Additional 4xx cases
    #[allow(clippy::cognitive_complexity)]
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
        let result: Result<i32, _> = with_retry(|| async { Err(anyhow::anyhow!("persistent error")) }).await;
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

    // ── ExceptionClassifier: Rate Limit Patterns ─────────────────────────────────

    #[test_case("rate limit exceeded", ProviderError::RateLimit { retry_after_secs: None }, true; "rate_limit_exceeded")]
    #[test_case("Rate limit: too many requests", ProviderError::RateLimit { retry_after_secs: None }, true; "rate_limit_colon")]
    #[test_case("rate_limit exceeded", ProviderError::RateLimit { retry_after_secs: None }, true; "rate_underscore_exceeded")]
    #[test_case("429 Too Many Requests", ProviderError::RateLimit { retry_after_secs: None }, true; "429_too_many")]
    #[test_case("quota exceeded", ProviderError::RateLimit { retry_after_secs: None }, true; "quota_exceeded")]
    #[test_case("quota limit exceeded", ProviderError::RateLimit { retry_after_secs: None }, true; "quota_limit_exceeded")]
    #[test_case("monthly quota exceeded", ProviderError::RateLimit { retry_after_secs: None }, true; "monthly_quota_exceeded")]
    #[test_case("daily quota exceeded", ProviderError::RateLimit { retry_after_secs: None }, true; "daily_quota_exceeded")]
    #[test_case("service tier limit", ProviderError::RateLimit { retry_after_secs: None }, true; "service_tier_limit")]
    #[test_case("limit exceeded", ProviderError::RateLimit { retry_after_secs: None }, true; "limit_exceeded")]
    #[test_case("try again later", ProviderError::RateLimit { retry_after_secs: None }, true; "try_again_later")]
    #[test_case("try again in a moment", ProviderError::RateLimit { retry_after_secs: None }, true; "try_again_moment")]
    #[test_case("retry after 60 seconds", ProviderError::RateLimit { retry_after_secs: Some(60) }, true; "retry_60_secs")]
    fn classify_rate_limit_errors(msg: &str, expected: ProviderError, _retryable: bool) {
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify(msg);
        match (&expected, &result) {
            (ProviderError::RateLimit { .. }, ProviderError::RateLimit { .. }) => {
                // Both are RateLimit variants
            }
            _ => panic!("Expected RateLimit, got {:?}", result),
        }
        assert!(result.is_retryable());
    }

    // ── ExceptionClassifier: Context Window Patterns ─────────────────────────────

    #[test_case("context length exceeded", ProviderError::ContextLength(0), false)]
    #[test_case("exceed context window", ProviderError::ContextLength(0), false)]
    #[test_case("exceeds maximum context", ProviderError::ContextLength(0), false)]
    #[test_case("maximum context length", ProviderError::ContextLength(0), false)]
    #[test_case("context limit exceeded", ProviderError::ContextLength(0), false)]
    #[test_case("too many tokens", ProviderError::ContextLength(0), false)]
    #[test_case("tokens exceeded 128000", ProviderError::ContextLength(128000), false)]
    #[test_case("token limit exceeded", ProviderError::ContextLength(0), false)]
    #[test_case("input too long for model", ProviderError::ContextLength(0), false)]
    #[test_case("message too long", ProviderError::ContextLength(0), false)]
    #[test_case("max tokens exceeded", ProviderError::ContextLength(0), false)]
    #[test_case("input length limit", ProviderError::ContextLength(0), false)]
    #[test_case("total length exceeds limit", ProviderError::ContextLength(0), false)]
    #[test_case("200000 tokens exceeds model limit", ProviderError::ContextLength(200000), false)]
    fn classify_context_window_errors(msg: &str, expected: ProviderError, _retryable: bool) {
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify(msg);
        match (&expected, &result) {
            (ProviderError::ContextLength(_), ProviderError::ContextLength(_)) => {
                // Both are ContextLength variants
            }
            _ => panic!("Expected ContextLength, got {:?}", result),
        }
        assert!(!result.is_retryable());
    }

    // ── ExceptionClassifier: Content Policy Violations ───────────────────────────

    #[test_case("content policy violation", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("policy_error: harmful content detected", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("content violates safety policy", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("safety filter triggered", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("harmful content blocked", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("inappropriate content", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("restricted content", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("cannot assist with that request", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("unable to provide this content", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("blocked by content policy", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("moderation filter triggered", ProviderError::Source(anyhow::anyhow!("")), false)]
    #[test_case("flagged for review", ProviderError::Source(anyhow::anyhow!("")), false)]
    fn classify_content_policy_errors(msg: &str, _expected: ProviderError, _retryable: bool) {
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify(msg);
        // Content policy violations should not be retryable
        assert!(
            !result.is_retryable(),
            "Content policy error should not be retryable"
        );
    }

    // ── ExceptionClassifier: Network Errors ─────────────────────────────────────

    #[test_case("connection refused", true)]
    #[test_case("connection reset", true)]
    #[test_case("connection timed out", true)]
    #[test_case("network error: host unreachable", true)]
    #[test_case("network failure", true)]
    #[test_case("failed to connect", true)]
    #[test_case("host not found", true)]
    #[test_case("dns resolution failed", true)]
    #[test_case("socket error", true)]
    fn classify_network_errors(msg: &str, retryable: bool) {
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify(msg);
        assert!(matches!(result, ProviderError::Network(_)));
        assert_eq!(result.is_retryable(), retryable);
    }

    // ── ExceptionClassifier: Timeout Errors ──────────────────────────────────────

    #[test_case("request timed out", true)]
    #[test_case("timeout error", true)]
    #[test_case("operation timed out", true)]
    #[test_case("timed out waiting for response", true)]
    fn classify_timeout_errors(msg: &str, retryable: bool) {
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify(msg);
        assert!(matches!(result, ProviderError::Timeout));
        assert_eq!(result.is_retryable(), retryable);
    }

    // ── ExceptionClassifier: Auth Errors ────────────────────────────────────────

    #[test_case("401 Unauthorized", true; "unauthorized_401")]
    #[test_case("403 Forbidden", true; "forbidden_403")]
    #[test_case("invalid api key", true; "invalid_api_key_space")]
    #[test_case("invalid api_key", true; "invalid_api_key_underscore")]
    #[test_case("authentication failed", true; "auth_failed")]
    #[test_case("auth error", true; "auth_error")]
    #[test_case("api key required", true; "api_key_required")]
    #[test_case("unauthorized access", true; "unauthorized_access")]
    fn classify_auth_errors(msg: &str, retryable: bool) {
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify(msg);
        assert!(matches!(result, ProviderError::Auth(_)));
        assert_eq!(result.is_retryable(), retryable);
    }

    // ── ExceptionClassifier: Server Errors ──────────────────────────────────────

    #[test_case("500 Internal Server Error", true)]
    #[test_case("502 Bad Gateway", true)]
    #[test_case("503 Service Unavailable", true)]
    #[test_case("504 Gateway Timeout", true)]
    #[test_case("server overloaded", true)]
    #[test_case("internal server error", true)]
    #[test_case("bad gateway", true)]
    #[test_case("service unavailable", true)]
    #[test_case("temporary failure", true)]
    fn classify_server_errors(msg: &str, retryable: bool) {
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify(msg);
        assert!(matches!(result, ProviderError::Server(_, _)));
        assert_eq!(result.is_retryable(), retryable);
    }

    // ── ExceptionClassifier: is_retryable integration ──────────────────────────

    #[test]
    fn is_retryable_uses_classifier_for_unknown_errors() {
        // Unknown errors should be wrapped in Source and treated as retryable (conservative)
        let err = anyhow::anyhow!("some unknown error");
        assert!(
            is_retryable(&err),
            "Unknown errors should be retryable by default"
        );

        // Context window errors should NOT be retryable
        let err = anyhow::anyhow!("context window exceeded");
        assert!(!is_retryable(&err));

        // Content policy errors should NOT be retryable
        let err = anyhow::anyhow!("content policy violation");
        assert!(!is_retryable(&err));

        // Rate limit errors should be retryable
        let err = anyhow::anyhow!("rate limit exceeded");
        assert!(is_retryable(&err));

        // Network errors should be retryable
        let err = anyhow::anyhow!("connection refused");
        assert!(is_retryable(&err));
    }

    // ── ExceptionClassifier: Priority ordering ───────────────────────────────────

    #[test]
    fn classify_priority_context_window_before_rate_limit() {
        // If a message mentions both context and rate limit, context should take priority
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify("context window exceeded despite rate limit");
        // Context length is more specific and checked first
        assert!(matches!(result, ProviderError::ContextLength(_)));
    }

    #[test]
    fn classify_priority_content_policy_before_rate_limit() {
        // If a message mentions both content policy and rate limit, content policy should take priority
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify("content policy violation after rate limit wait");
        // Content policy errors are fatal and checked before rate limits
        assert!(!result.is_retryable());
    }

    // ── ExceptionClassifier: Edge cases ─────────────────────────────────────────

    #[test]
    fn classify_empty_message() {
        let classifier = ExceptionClassifier::new();
        let result = classifier.classify("");
        // Empty message should fallback to Source (retryable)
        assert!(result.is_retryable());
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn classify_case_insensitive() {
        let classifier = ExceptionClassifier::new();

        // Test various cases for rate limit
        assert!(matches!(
            classifier.classify("RATE LIMIT"),
            ProviderError::RateLimit { .. }
        ));
        assert!(matches!(
            classifier.classify("Rate Limit"),
            ProviderError::RateLimit { .. }
        ));
        assert!(matches!(
            classifier.classify("RateLimit"),
            ProviderError::RateLimit { .. }
        ));

        // Test various cases for context window
        assert!(matches!(
            classifier.classify("CONTEXT LENGTH"),
            ProviderError::ContextLength(_)
        ));
        assert!(matches!(
            classifier.classify("Context Length"),
            ProviderError::ContextLength(_)
        ));
    }

    #[test]
    fn classify_extracts_http_codes() {
        let classifier = ExceptionClassifier::new();

        let result = classifier.classify("Got 401 error: unauthorized");
        assert!(matches!(result, ProviderError::Auth(401)));

        let result = classifier.classify("Server returned 503");
        assert!(matches!(result, ProviderError::Server(503, _)));

        let result = classifier.classify("Rate limited with 429");
        assert!(matches!(result, ProviderError::RateLimit { .. }));
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // RetryPolicy Tests
    // ═══════════════════════════════════════════════════════════════════════════════

    // ── RetryPolicy: Basic structure ─────────────────────────────────────────────

    #[test]
    fn retry_policy_default_is_empty() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.rate_limit_retries, None);
        assert_eq!(policy.timeout_retries, None);
        assert_eq!(policy.context_window_retries, None);
        assert_eq!(policy.bad_request_retries, None);
    }

    #[test]
    fn retry_policy_builder_pattern() {
        let policy = RetryPolicy::new()
            .with_rate_limit_retries(Some(10))
            .with_timeout_retries(Some(5))
            .with_context_window_retries(Some(0))
            .with_bad_request_retries(Some(2));

        assert_eq!(policy.rate_limit_retries, Some(10));
        assert_eq!(policy.timeout_retries, Some(5));
        assert_eq!(policy.context_window_retries, Some(0));
        assert_eq!(policy.bad_request_retries, Some(2));
    }

    // ── RetryPolicy: max_retries_for typed errors ───────────────────────────────

    #[test]
    fn retry_policy_rate_limit_error() {
        let policy = RetryPolicy::new().with_rate_limit_retries(Some(5));
        let err: anyhow::Error = ProviderError::RateLimit { retry_after_secs: None }.into();
        assert_eq!(policy.max_retries_for(&err), Some(5));
    }

    #[test]
    fn retry_policy_timeout_error() {
        let policy = RetryPolicy::new().with_timeout_retries(Some(3));
        let err: anyhow::Error = ProviderError::Timeout.into();
        assert_eq!(policy.max_retries_for(&err), Some(3));
    }

    #[test]
    fn retry_policy_context_length_error() {
        let policy = RetryPolicy::new().with_context_window_retries(Some(1));
        let err: anyhow::Error = ProviderError::ContextLength(128_000).into();
        assert_eq!(policy.max_retries_for(&err), Some(1));
    }

    #[test]
    fn retry_policy_auth_error() {
        let policy = RetryPolicy::new().with_bad_request_retries(Some(2));
        let err: anyhow::Error = ProviderError::Auth(401).into();
        assert_eq!(policy.max_retries_for(&err), Some(2));
    }

    #[test]
    fn retry_policy_server_error_uses_global() {
        // Server errors don't have a specific policy field, so they should return None
        let policy = RetryPolicy::new()
            .with_rate_limit_retries(Some(10))
            .with_timeout_retries(Some(5));
        let err: anyhow::Error = ProviderError::Server(500, "test".into()).into();
        assert_eq!(policy.max_retries_for(&err), None); // None = use global config
    }

    #[test]
    fn retry_policy_network_error_uses_global() {
        // Network errors don't have a specific policy field, so they should return None
        let policy = RetryPolicy::new();
        let err: anyhow::Error = ProviderError::Network("connection refused".into()).into();
        assert_eq!(policy.max_retries_for(&err), None); // None = use global config
    }

    // ── RetryPolicy: Non-retryable errors return None ─────────────────────────────

    #[test]
    fn retry_policy_non_retryable_returns_none() {
        let policy = RetryPolicy::new()
            .with_rate_limit_retries(Some(10))
            .with_timeout_retries(Some(5))
            .with_context_window_retries(Some(1))
            .with_bad_request_retries(Some(2));

        // ContextLength is not retryable
        let err: anyhow::Error = ProviderError::ContextLength(128_000).into();
        // The policy says we can retry it, but is_retryable says no
        assert!(!is_retryable(&err));
        assert_eq!(policy.max_retries_for(&err), None);
    }

    // ── RetryPolicy: max_retries_for with None (use global) ──────────────────────

    #[test]
    fn retry_policy_none_means_use_global() {
        let policy = RetryPolicy::new(); // All fields are None
        let err: anyhow::Error = ProviderError::RateLimit { retry_after_secs: None }.into();
        // None means "use global config", but is_retryable must also pass
        assert!(is_retryable(&err));
        assert_eq!(policy.max_retries_for(&err), None);
    }

    // ── RetryPolicy: Zero retries disables that error type ───────────────────────

    #[test]
    fn retry_policy_zero_retries_disables_error_type() {
        let policy = RetryPolicy::new()
            .with_rate_limit_retries(Some(0)) // Disable rate limit retries
            .with_timeout_retries(Some(5));

        let err: anyhow::Error = ProviderError::RateLimit { retry_after_secs: None }.into();
        assert_eq!(policy.max_retries_for(&err), Some(0));
    }

    // ── RetryPolicy: String-based errors ──────────────────────────────────────────

    #[test]
    fn retry_policy_string_based_rate_limit() {
        let policy = RetryPolicy::new().with_rate_limit_retries(Some(7));
        let err = anyhow::anyhow!("rate limit exceeded");
        // String-based errors are classified by ExceptionClassifier
        // which identifies "rate limit" as a RateLimit error
        let max_retries = policy.max_retries_for(&err);
        assert_eq!(max_retries, Some(7));
    }

    #[test]
    fn retry_policy_string_based_timeout() {
        let policy = RetryPolicy::new().with_timeout_retries(Some(3));
        let err = anyhow::anyhow!("request timed out");
        let max_retries = policy.max_retries_for(&err);
        assert_eq!(max_retries, Some(3));
    }

    // ── RetryPolicy: Clone and Debug ─────────────────────────────────────────────

    #[test]
    fn retry_policy_is_cloneable() {
        let policy = RetryPolicy::new()
            .with_rate_limit_retries(Some(10))
            .with_timeout_retries(Some(5));
        let cloned = policy.clone();
        assert_eq!(cloned.rate_limit_retries, Some(10));
        assert_eq!(cloned.timeout_retries, Some(5));
    }

    #[test]
    fn retry_policy_has_debug_representation() {
        let policy = RetryPolicy::new().with_rate_limit_retries(Some(10));
        let debug_str = format!("{:?}", policy);
        assert!(debug_str.contains("RetryPolicy"));
        assert!(debug_str.contains("rate_limit_retries"));
    }

    // ── with_retry_policy: Integration tests ─────────────────────────────────────

    #[tokio::test]
    async fn with_retry_policy_succeeds_on_first_attempt() {
        let config = RetryConfig::new(5, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new();

        let result = with_retry_policy(|| async { Ok::<_, Error>(42) }, &config, &policy).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn with_retry_policy_respects_rate_limit_retries() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig::new(10, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new().with_rate_limit_retries(Some(2)); // Only 2 retries for rate limit

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::anyhow!("rate limit exceeded"))
                }
            },
            &config,
            &policy,
        )
        .await;

        assert!(result.is_err());
        // Should have initial + 2 retries = 3 attempts (not 10 from global config)
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn with_retry_policy_respects_timeout_retries() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig::new(10, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new().with_timeout_retries(Some(1)); // Only 1 retry for timeout

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::anyhow!("request timed out"))
                }
            },
            &config,
            &policy,
        )
        .await;

        assert!(result.is_err());
        // Should have initial + 1 retry = 2 attempts
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn with_retry_policy_does_not_retry_context_length() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig::new(5, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        // Even with context_window_retries set, the error is not retryable
        let policy = RetryPolicy::new().with_context_window_retries(Some(3));

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::anyhow!("context window exceeded"))
                }
            },
            &config,
            &policy,
        )
        .await;

        assert!(result.is_err());
        // Should have only 1 attempt because context window errors are not retryable
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn with_retry_policy_succeeds_after_policy_retries() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig::new(10, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        let policy = RetryPolicy::new().with_rate_limit_retries(Some(5));

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n < 3 {
                        Err(anyhow::anyhow!("rate limit"))
                    } else {
                        Ok::<_, Error>(42)
                    }
                }
            },
            &config,
            &policy,
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 4); // 3 failures + 1 success
    }

    #[tokio::test]
    async fn with_retry_policy_different_error_types_independent() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig::new(10, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        // Rate limit: 1 retry, Timeout: 2 retries
        let policy = RetryPolicy::new()
            .with_rate_limit_retries(Some(1))
            .with_timeout_retries(Some(2));

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    match n {
                        0 => Err(anyhow::anyhow!("rate limit")), // retry
                        1 => Err(anyhow::anyhow!("timeout")),    // retry
                        2 => Err(anyhow::anyhow!("timeout")),    // retry
                        3 => Err(anyhow::anyhow!("rate limit")), // stops here (rate limit exceeded its 1 retry)
                        _ => Ok::<_, Error>(42),
                    }
                }
            },
            &config,
            &policy,
        )
        .await;

        assert!(result.is_err());
        // 4 attempts: rate limit, timeout, timeout, rate limit (stops)
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn with_retry_policy_zero_retries_disables_error_type() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig::new(10, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        // Disable rate limit retries
        let policy = RetryPolicy::new().with_rate_limit_retries(Some(0));

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::anyhow!("rate limit exceeded"))
                }
            },
            &config,
            &policy,
        )
        .await;

        assert!(result.is_err());
        // Should have only 1 attempt because rate limit retries are disabled
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn with_retry_policy_global_config_fallback() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Global config allows 3 attempts
        let config = RetryConfig::new(3, Duration::from_millis(1), Duration::from_secs(1), 1.0);
        // No specific policy for server errors (they use global config)
        let policy = RetryPolicy::new().with_rate_limit_retries(Some(10));

        let result: Result<i32, anyhow::Error> = with_retry_policy(
            move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::anyhow!("500 internal server error"))
                }
            },
            &config,
            &policy,
        )
        .await;

        assert!(result.is_err());
        // Should have 3 attempts (global config) for server errors
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
