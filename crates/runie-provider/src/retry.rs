//! Retry configuration for provider streams.
//!
//! Uses `reqwest_eventsource`'s built-in retry policy for HTTP-level retries.
//! The retry behavior is: retry transient failures with exponential backoff,
//! but only *before* the stream starts emitting events. Once the stream has
//! started, any error is surfaced immediately.

use anyhow::Error;
use futures::Future;
use std::time::Duration;

/// Base delay for exponential backoff.
const BASE_DELAY: Duration = Duration::from_millis(500);

/// Determines if an error should trigger a retry.
pub fn is_retryable(e: &Error) -> bool {
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn with_retry_succeeds_on_first_attempt() {
        let result = with_retry(|| async { Ok::<_, Error>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn with_retry_fails_after_max_attempts() {
        let result: Result<i32, _> = with_retry(|| async {
            Err(anyhow::anyhow!("persistent error"))
        })
        .await;
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
        assert_eq!(counter.load(Ordering::SeqCst), 2);
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
}
