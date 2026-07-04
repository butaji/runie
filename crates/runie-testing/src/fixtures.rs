//! Common test fixtures.

pub mod anthropic;
pub mod grok_build;
pub mod minimax;
pub mod openai;

use std::sync::Arc;

use runie_core::config::Config;
use runie_core::permissions::{AutoAllowSink, PermissionGate, PermissionManager};
use runie_core::session::store::SessionStore;
use tempfile::TempDir;

use crate::env_lock::with_env;

/// Guard that restores HOME when dropped.
pub struct HomeRestore {
    original_home: Option<std::ffi::OsString>,
}

impl HomeRestore {
    fn set(dir: &std::path::Path) -> Self {
        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", dir);
        Self { original_home }
    }
}

impl Drop for HomeRestore {
    fn drop(&mut self) {
        match &self.original_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
    }
}

/// Create an isolated temp home directory and set `HOME` to it.
///
/// The original `HOME` is restored when the returned `TempDir` is dropped.
pub fn temp_home() -> (TempDir, HomeRestore) {
    let dir = TempDir::new().unwrap();
    let restore = HomeRestore::set(dir.path());
    (dir, restore)
}

/// Build a default config rooted in the temp home.
pub fn load_default_config_for_test(test_home: &TempDir) -> Config {
    with_env(|env| {
        env.set("HOME", test_home.path().to_str().unwrap_or("/tmp"));
        let path = test_home.path().join(".runie").join("config.toml");
        Config::load(Some(&path))
    })
}

/// Return a mock provider suitable for deterministic tests.
pub fn mock_provider() -> runie_provider::BuiltProvider {
    with_env(|env| {
        env.set("RUNIE_MOCK", "1");
        let mock = runie_provider::MockProvider::default();
        runie_provider::BuiltProvider::from_provider(Box::new(mock), "mock", "echo")
    })
}

/// Build a permission gate that allows all operations without prompting.
pub fn allow_all_gate() -> PermissionGate {
    PermissionGate::new(PermissionManager::default(), Arc::new(AutoAllowSink))
}

/// Build a session store inside the temp home.
pub fn session_store_for_test(test_home: &TempDir) -> SessionStore {
    let dir = test_home.path().join(".runie").join("sessions");
    SessionStore::new(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_allow_all_gate_is_default_manager() {
        let gate = allow_all_gate();
        // The gate should have a sink reference
        let _sink = gate.sink_ref();
        // sink_ref returns &Arc<dyn ApprovalSink>
    }

    #[test]
    fn shared_mock_provider_returns_built() {
        let provider = mock_provider();
        // Should be able to get key and model info
        assert_eq!(provider.key(), "mock");
        assert_eq!(provider.model(), "echo");
    }

    #[test]
    fn temp_home_isolates_home() {
        let original_home = std::env::var_os("HOME");
        {
            let (_dir, _restore) = temp_home();
            // HOME should be different inside
            let current = std::env::var_os("HOME").unwrap();
            assert!(!current.is_empty());
        }
        // HOME should be restored after drop
        let after = std::env::var_os("HOME");
        assert_eq!(after, original_home);
    }
}
