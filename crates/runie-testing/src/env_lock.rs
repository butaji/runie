//! Centralized environment lock for serializing tests that touch environment variables.

use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::sync::Mutex;

/// Global lock to serialize tests that touch environment variables.
///
/// All crates that need to modify environment variables during tests
/// should use this lock to avoid race conditions in parallel test execution.
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Guard that restores environment variables when dropped.
pub struct EnvRestore {
    vars: HashMap<String, Option<OsString>>,
}

impl EnvRestore {
    /// Create a new empty restore guard.
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// Set an environment variable, saving its prior value for restoration.
    pub fn set(&mut self, key: &str, value: impl AsRef<std::ffi::OsStr>) {
        let prior = env::var_os(key);
        self.vars.insert(key.to_string(), prior);
        env::set_var(key, value.as_ref());
    }

    /// Remove an environment variable, saving its prior value for restoration.
    pub fn remove(&mut self, key: &str) {
        let prior = env::var_os(key);
        self.vars.insert(key.to_string(), prior);
        env::remove_var(key);
    }
}

impl Default for EnvRestore {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (key, prior) in self.vars.drain() {
            match prior {
                Some(v) => env::set_var(&key, v),
                None => env::remove_var(&key),
            }
        }
    }
}

/// RAII guard returned by [`with_env`].
pub struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    _restore: EnvRestore,
}

/// Execute a closure with protected environment variable mutations.
///
/// All mutations inside the closure are automatically reverted when the
/// closure returns or panics. External code does not need to manually
/// clean up.
///
/// This also acquires `ENV_LOCK` to serialize with other tests that
/// touch environment variables.
///
/// # Example
/// ```ignore
/// let result = with_env(|env| {
///     env.set("HOME", "/tmp/test-home");
///     env.set("RUNIE_MOCK", "1");
///     // do something
/// });
/// ```
pub fn with_env<F, T>(f: F) -> T
where
    F: FnOnce(&mut EnvRestore) -> T,
{
    let _lock = ENV_LOCK.lock().unwrap();
    let mut restore = EnvRestore::new();
    f(&mut restore)
}

/// Execute a closure while the environment lock is held.
///
/// Unlike [`with_env`], this does not automatically restore environment
/// variables. Prefer `with_env` for most use cases.
pub fn env_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = ENV_LOCK.lock().unwrap();
    f()
}

use std::sync::MutexGuard;
