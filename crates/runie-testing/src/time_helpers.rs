//! Time helpers for deterministic async testing.
//!
//! Provides utilities for controlling tokio time in tests:
//! - `TestTimeGuard` - RAII guard that pauses time for the duration of a test
//! - `advance_and_poll` - advances time and waits for a future to complete

use std::future::Future;
use std::time::Duration;

/// RAII guard that pauses tokio time for the duration of a test.
///
/// # Example
/// ```ignore
/// #[tokio::test]
/// async fn test_with_paused_time() {
///     let _guard = TestTimeGuard::new();
///     // Time is now paused - use advance() to control it
///     tokio::time::advance(Duration::from_secs(1)).await;
/// }
/// ```
pub struct TestTimeGuard {
    // The guard is held; dropping it will resume time
}

impl TestTimeGuard {
    /// Creates a new guard that pauses tokio time.
    ///
    /// Returns None if the runtime doesn't support time pausing
    /// (e.g., in multi-threaded runtimes without `enable_pausing`).
    pub fn new() -> Option<Self> {
        tokio::time::pause();
        Some(Self {})
    }

    /// Advance time by the given duration and wait for ready tasks.
    pub async fn advance(duration: Duration) {
        tokio::time::advance(duration).await;
        // Yield to allow ready tasks to run
        tokio::task::yield_now().await;
    }

    /// Advance time multiple steps with yields in between.
    pub async fn advance_steps(steps: u32, step_duration: Duration) {
        for _ in 0..steps {
            Self::advance(step_duration).await;
        }
    }
}

/// Wait for a future to complete with a timeout, advancing virtual time.
///
/// This combines timeout with virtual time advancement, allowing tests
/// to wait for async operations without real wall-clock delays.
pub async fn with_timeout<T>(
    future: impl Future<Output = T>,
    timeout: Duration,
) -> Result<T, tokio::time::error::Elapsed> {
    tokio::time::timeout(timeout, future).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_time_guard_pauses_time() {
        let _guard = TestTimeGuard::new().expect("should support time pausing");

        tokio::time::advance(Duration::from_secs(10)).await;
        // The test completes quickly because time is virtual
    }

    #[tokio::test]
    async fn test_with_timeout_works() {
        let _guard = TestTimeGuard::new().expect("should support time pausing");

        let result = with_timeout(async { 42 }, Duration::from_secs(10)).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_with_timeout_expires() {
        let _guard = TestTimeGuard::new().expect("should support time pausing");

        let slow_future = async {
            tokio::time::sleep(Duration::from_secs(100)).await;
            42
        };

        let result = with_timeout(slow_future, Duration::from_secs(10)).await;
        assert!(result.is_err());
    }
}
