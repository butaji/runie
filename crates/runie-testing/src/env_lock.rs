//! Centralized environment lock for serializing tests that touch environment variables.

use std::sync::Mutex;

/// Global lock to serialize tests that touch environment variables.
/// 
/// All crates that need to modify environment variables during tests
/// should use this lock to avoid race conditions in parallel test execution.
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Acquire the environment lock, execute the closure, and return the result.
/// 
/// # Example
/// ```ignore
/// let result = env_lock(|| {
///     std::env::set_var("KEY", "value");
///     // do something
/// });
/// ```
pub fn env_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = ENV_LOCK.lock().unwrap();
    f()
}
