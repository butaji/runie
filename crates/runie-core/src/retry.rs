//! Retry with exponential backoff for transient errors.

use anyhow::Error;
use std::future::Future;
use std::time::Duration;

/// Configuration for retry behaviour.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retry attempts.
    pub max_attempts: u32,
    /// Base delay before first retry.
    pub base_delay: Duration,
    /// Maximum delay cap.
    pub max_delay: Duration,
    /// Random jitter added to each delay.
    pub jitter: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            jitter: Duration::from_millis(100),
        }
    }
}

/// Whether an error should be retried.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RetryableError {
    RateLimited,
    ServerError(u16),
    NetworkError,
}

impl RetryableError {
    /// Classify an HTTP status code.
    pub fn from_status(status: u16) -> Option<Self> {
        match status {
            429 => Some(Self::RateLimited),
            500..=599 => Some(Self::ServerError(status)),
            _ => None,
        }
    }
}

/// Classify a provider error as retryable or fatal.
pub fn classify_provider_error(err: &Error) -> Option<RetryableError> {
    let _msg = err.to_string();
    let msg = err.to_string();
    // reqwest network errors
    if msg.contains("connection")
        || msg.contains("timeout")
        || msg.contains("network")
        || msg.contains("tcp")
    {
        return Some(RetryableError::NetworkError);
    }
    // parse status from "status: 429" or "429"
    if let Some(pos) = msg
        .find(|c: char| c.is_ascii_digit())
        .filter(|_| msg.len() >= 3)
    {
        let slice = &msg[pos..pos + 3];
        if let Ok(status) = slice.parse::<u16>() {
            return RetryableError::from_status(status);
        }
    }
    None
}

/// Retry a fallible async operation with exponential backoff.
///
/// Calls `f()` up to `config.max_attempts` times. If `f()` returns an error
/// that `classify` marks as retryable, waits with exponential backoff and
/// retries. Returns the first successful result or the final error.
pub async fn with_retry<F, Fut, T, C>(
    config: &RetryConfig,
    mut classify: C,
    mut f: F,
) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
    C: FnMut(&Error) -> Option<RetryableError>,
{
    let mut attempt = 0u32;

    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(err) => {
                attempt += 1;
                match classify(&err) {
                    Some(retryable) => {
                        if attempt >= config.max_attempts {
                            return Err(anyhow::anyhow!(
                                "operation failed after {} attempts ({:?})",
                                attempt,
                                retryable
                            ));
                        }
                        let delay = compute_delay(config, attempt);
                        tokio::time::sleep(delay).await;
                    }
                    None => return Err(err),
                }
            }
        }
    }
}

fn compute_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let exponential = config.base_delay * 2u32.saturating_pow(attempt);
    let capped = exponential.min(config.max_delay);
    if config.jitter.is_zero() {
        return capped;
    }
    let jitter_max = config.jitter.as_millis() as u64;
    let jitter_ms = (rand_u64() % jitter_max).max(jitter_max / 2);
    capped + Duration::from_millis(jitter_ms)
}

// Simple deterministic-ish pseudo-random u64
fn rand_u64() -> u64 {
    use std::time::Instant;
    let n = Instant::now();
    ((n.elapsed().as_nanos() as u64) ^ (std::process::id() as u64).wrapping_mul(0x517cc1b727220a95))
        .wrapping_add(0x9e3779b97f4a7c15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retries_on_rate_limit() {
        let count = std::cell::Cell::new(0u32);
        let result: Result<i32, anyhow::Error> = with_retry(
            &RetryConfig {
                max_attempts: 3,
                base_delay: Duration::from_millis(1),
                max_delay: Duration::from_millis(10),
                jitter: Duration::ZERO,
            },
            |err| classify_provider_error(err),
            || async {
                count.set(count.get() + 1);
                if count.get() < 3 {
                    Err(anyhow::anyhow!("429 rate limited"))
                } else {
                    Ok(count.get() as i32)
                }
            },
        )
        .await;
        assert_eq!(result.unwrap(), 3);
    }

    #[tokio::test]
    async fn fatal_error_no_retry() {
        let count = std::cell::Cell::new(0u32);
        let result: Result<(), anyhow::Error> = with_retry(
            &RetryConfig {
                max_attempts: 3,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
                jitter: Duration::ZERO,
            },
            |_err: &Error| None::<RetryableError>,
            || async {
                count.set(count.get() + 1);
                Err(anyhow::anyhow!("fatal error"))
            },
        )
        .await;
        assert!(result.is_err());
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn retryable_error_from_status() {
        assert_eq!(
            RetryableError::from_status(429),
            Some(RetryableError::RateLimited)
        );
        assert_eq!(
            RetryableError::from_status(500),
            Some(RetryableError::ServerError(500))
        );
        assert_eq!(
            RetryableError::from_status(502),
            Some(RetryableError::ServerError(502))
        );
        assert_eq!(RetryableError::from_status(200), None);
        assert_eq!(RetryableError::from_status(400), None);
    }

    #[test]
    fn retry_config_default() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_attempts, 3);
        assert_eq!(cfg.base_delay, Duration::from_millis(500));
        assert_eq!(cfg.max_delay, Duration::from_secs(30));
    }

    #[test]
    fn classify_provider_error_rate_limit() {
        let err = anyhow::anyhow!("429");
        assert_eq!(
            classify_provider_error(&err),
            Some(RetryableError::RateLimited)
        );
    }

    #[test]
    fn classify_provider_error_network() {
        let err = anyhow::anyhow!("connection reset");
        assert_eq!(
            classify_provider_error(&err),
            Some(RetryableError::NetworkError)
        );
    }

    #[test]
    fn classify_provider_error_fatal() {
        let err = anyhow::anyhow!("invalid request: missing API key");
        assert_eq!(classify_provider_error(&err), None);
    }
}
