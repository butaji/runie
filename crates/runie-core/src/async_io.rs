//! Helpers for moving blocking work off the async runtime.

use tokio::runtime::Handle;
use tokio::task::JoinHandle;

/// Run a blocking closure on a Tokio blocking thread when a runtime is present.
/// When called outside a runtime the closure runs synchronously.
///
/// This is a tactical helper for legacy synchronous call sites that are invoked
/// from both async production code and plain unit tests. New code should prefer
/// explicit `spawn_blocking`/`block_on` boundaries.
pub fn run_blocking_if_runtime<F, T>(f: F) -> Option<JoinHandle<T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match Handle::try_current() { Ok(handle) => {
        Some(handle.spawn_blocking(f))
    } _ => {
        let _ = f();
        None
    }}
}

/// Run a blocking closure without blocking the async runtime.
///
/// Uses `tokio::task::block_in_place` when a runtime is present; otherwise
/// runs synchronously. This is for short, unavoidable synchronous calls (file
/// reads, config lookups) that are invoked from code paths that may be reached
/// from an async actor but cannot easily be made async themselves.
pub fn block_in_place_if_runtime<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    if let Ok(handle) = Handle::try_current() {
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread {
            return tokio::task::block_in_place(f);
        }
    }
    f()
}

#[cfg(test)]
mod tests {
    use super::{block_in_place_if_runtime, run_blocking_if_runtime};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn runs_synchronously_without_runtime() {
        let ran = Arc::new(AtomicUsize::new(0));
        let ran2 = ran.clone();
        let result = run_blocking_if_runtime(move || {
            ran2.fetch_add(1, Ordering::SeqCst);
            42
        });
        assert!(result.is_none(), "should not spawn without runtime");
        assert_eq!(ran.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn runs_on_blocking_thread_with_runtime() {
        let ran = Arc::new(AtomicUsize::new(0));
        let ran2 = ran.clone();
        let handle = run_blocking_if_runtime(move || {
            ran2.fetch_add(1, Ordering::SeqCst);
            42
        });
        assert!(handle.is_some(), "should spawn with runtime");
        let value = handle.unwrap().await.expect("task completed");
        assert_eq!(value, 42);
        assert_eq!(ran.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn block_in_place_runs_synchronously_without_runtime() {
        let value = block_in_place_if_runtime(|| 7);
        assert_eq!(value, 7);
    }

    #[tokio::test]
    async fn block_in_place_runs_with_runtime() {
        let value = block_in_place_if_runtime(|| 7);
        assert_eq!(value, 7);
    }
}
