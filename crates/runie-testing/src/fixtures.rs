//! Common test fixtures.

pub mod minimax;

use std::sync::Arc;
use std::sync::Once;

use runie_core::config::Config;
use runie_core::permissions::{AutoAllowSink, PermissionGate, PermissionManager};
use runie_core::session_store::SessionStore;
use tempfile::TempDir;

static SET_MOCK_HOME: Once = Once::new();

/// Create an isolated temp home directory and set `HOME` to it.
pub fn temp_home() -> TempDir {
    let dir = TempDir::new().unwrap();
    SET_MOCK_HOME.call_once(|| {
        std::env::set_var("HOME", dir.path());
    });
    dir
}

/// Build a default config rooted in the temp home.
pub fn load_default_config_for_test(test_home: &TempDir) -> Config {
    std::env::set_var("HOME", test_home.path());
    let path = test_home.path().join(".runie").join("config.toml");
    Config::load(Some(&path))
}

/// Return a mock provider suitable for deterministic tests.
pub fn mock_provider() -> runie_provider::DynProvider {
    let prev = std::env::var_os("RUNIE_MOCK");
    std::env::set_var("RUNIE_MOCK", "1");
    let mock = runie_provider::MockProvider::default();
    let provider = runie_provider::DynProvider::from_provider(Box::new(mock), "mock", "echo");
    // Restore prior state so tests don't pollute the env for subsequent tests.
    match prev {
        Some(v) => std::env::set_var("RUNIE_MOCK", v),
        None => std::env::remove_var("RUNIE_MOCK"),
    }
    provider
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
    fn shared_mock_provider_returns_dyn() {
        let provider = mock_provider();
        // Should be able to get key and model info
        assert_eq!(provider.key(), "mock");
        assert_eq!(provider.model(), "echo");
    }
}
