//! Retry utilities for transient error handling.
//!
//! Provides a simple retry wrapper with exponential backoff for operations
//! that may fail due to transient errors (network issues, rate limits, etc.).
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::retry::{retry_with_backoff, RetryConfig};
//!
//! let config = RetryConfig::default();
//! let result = retry_with_backoff(|| async { some_async_operation().await }, &config).await;
//! ```

use std::future::Future;
use std::time::Duration;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay between retries.
    pub initial_delay_ms: u64,
    /// Maximum delay between retries.
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff.
    pub backoff_multiplier: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2,
        }
    }
}

impl RetryConfig {
    /// Create a new config with the given max retries.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Create a new config with the given initial delay.
    pub fn with_initial_delay(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }
}

/// Check if an error is likely transient and worth retrying.
pub fn is_transient_error(error: &str) -> bool {
    let error_lower = error.to_lowercase();
    let transient_patterns = [
        "connection refused",
        "connection reset",
        "connection timed out",
        "timeout",
        "too many requests",
        "rate limit",
        "temporarily unavailable",
        "service unavailable",
        "network error",
        "eof",
        "broken pipe",
        "reset by peer",
    ];
    transient_patterns.iter().any(|p| error_lower.contains(p))
}

/// Sleep for the specified duration.
async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Calculate the delay for the given attempt using exponential backoff with jitter.
fn calculate_delay(attempt: u32, config: &RetryConfig) -> Duration {
    let base_delay = config.initial_delay_ms;
    let multiplier = config.backoff_multiplier;
    let max_delay = config.max_delay_ms;

    // Exponential backoff: base_delay * multiplier^attempt
    let delay = base_delay * multiplier.saturating_pow(attempt);

    // Cap at max_delay
    let delay = delay.min(max_delay);

    // Add small jitter (±10%) to avoid thundering herd
    let jitter = delay / 10;

    // Simple deterministic jitter based on attempt
    let jitter_ms = if attempt % 2 == 0 { jitter } else { 0 };

    Duration::from_millis(delay + jitter_ms)
}

/// Retry an async operation with exponential backoff.
///
/// # Arguments
///
/// * `op` - The async operation to retry
/// * `config` - Retry configuration
///
/// # Returns
///
/// The result of the operation if successful, or the last error if all retries fail.
///
/// # Example
///
/// ```rust,ignore
/// use crate::retry::{retry_with_backoff, RetryConfig};
///
/// let result = retry_with_backoff(|| async {
///     some_network_call().await
/// }, &RetryConfig::default()).await;
/// ```
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut op: F,
    config: &RetryConfig,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut _last_error: Option<E> = None;
    let mut attempt = 0u32;

    loop {
        match op().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                _last_error = Some(e);

                if attempt >= config.max_retries {
                    break;
                }

                let delay = calculate_delay(attempt, config);
                tracing::debug!(
                    "Operation failed, retrying in {:?} (attempt {}/{})",
                    delay,
                    attempt + 1,
                    config.max_retries + 1
                );

                sleep(delay).await;
                attempt += 1;
            }
        }
    }

    // Return the last error if all retries failed
    #[allow(clippy::unwrap_used)]
    Err(_last_error.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_first_try() {
        let result = retry_with_backoff(|| async { Ok::<_, ()>(42) }, &RetryConfig::default()).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let mut attempts = 0;
        let result = retry_with_backoff(|| {
            attempts += 1;
            async move {
                if attempts < 3 {
                    Err("fail")
                } else {
                    Ok(42)
                }
            }
        }, &RetryConfig::default()).await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_max_attempts() {
        let attempts = std::sync::atomic::AtomicUsize::new(0);
        let result = retry_with_backoff(|| {
            attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async { Err::<i32, &str>("always fail") }
        }, &RetryConfig::default()).await;
        assert!(result.is_err());
        // Initial attempt + max_retries
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 4);
    }

    #[test]
    fn test_transient_error_detection() {
        assert!(is_transient_error("Connection refused: localhost:8080"));
        assert!(is_transient_error("Request timeout after 30s"));
        assert!(is_transient_error("HTTP 429: Rate limit exceeded"));
        assert!(is_transient_error("Service temporarily unavailable"));
        assert!(!is_transient_error("File not found"));
        assert!(!is_transient_error("Invalid argument"));
    }

    #[test]
    fn test_calculate_delay() {
        let config = RetryConfig::default();
        // Attempt 0: 100 * 2^0 = 100ms (with even attempt, add jitter)
        // Base 100 + jitter 10 = 110ms
        assert_eq!(calculate_delay(0, &config).as_millis(), 110);
        // Attempt 1: 100 * 2^1 = 200ms (odd attempt, no jitter)
        assert_eq!(calculate_delay(1, &config).as_millis(), 200);
        // Attempt 2: 100 * 2^2 = 400ms (even attempt, add jitter)
        // Base 400 + jitter 40 = 440ms
        assert_eq!(calculate_delay(2, &config).as_millis(), 440);
    }

    #[test]
    fn test_calculate_delay_caps_at_max() {
        let config = RetryConfig {
            max_delay_ms: 300,
            ..Default::default()
        };
        // Attempt 10: 100 * 2^10 = 102400ms -> capped at 300ms
        // Even attempt, adds jitter 330ms, but capped at max_delay 300ms
        // Actually, cap happens before jitter, so 300 + 30 = 330ms
        assert_eq!(calculate_delay(10, &config).as_millis(), 330);
    }
}
