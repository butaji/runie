//! Test timeout helpers.
//!
//! Provides utilities for ensuring async tests don't hang forever:
//! - `with_timeout` — run a future with a timeout, returning an error on timeout
//! - `TimeoutError` — error type for timeout violations
//!
//! The workspace `[profile.test]` sets a global 60s timeout, but some tests
//! may need shorter or longer limits. Use these helpers for fine-grained control.

use std::future::Future;
use std::time::Duration;
use thiserror::Error;

/// Error returned when a test exceeds its timeout.
#[derive(Debug, Error)]
#[error("test timed out after {elapsed_secs:.1}s (limit: {limit_secs:.1}s)")]
pub struct TimeoutError {
    limit_secs: f64,
    elapsed_secs: f64,
}

impl TimeoutError {
    pub fn new(limit: Duration, elapsed: Duration) -> Self {
        Self {
            limit_secs: limit.as_secs_f64(),
            elapsed_secs: elapsed.as_secs_f64(),
        }
    }
}

/// Run a future with a timeout, returning an error if it exceeds the limit.
pub async fn with_timeout<F, T>(limit: Duration, future: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    let started = std::time::Instant::now();
    tokio::time::timeout(limit, future)
        .await
        .map_err(|_| TimeoutError::new(limit, started.elapsed()))
}

/// Run a future with a default 60s timeout.
pub async fn with_default_timeout<F, T>(future: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    with_timeout(Duration::from_secs(60), future).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn with_timeout_succeeds_within_limit() {
        let result = with_timeout(Duration::from_secs(5), async { 42 }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn with_timeout_fails_after_limit() {
        let result = with_timeout(Duration::from_millis(10), async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            42
        })
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.limit_secs >= 0.01);
    }
}
