//! Common test fixtures.

use std::sync::Once;

use runie_core::config::Config;
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
    std::env::set_var("RUNIE_MOCK", "1");
    runie_provider::DynProvider::new_with_config("mock", "echo", &runie_core::config::Config::default())
        .expect("mock provider available")
}

/// Build a session store inside the temp home.
pub fn session_store_for_test(test_home: &TempDir) -> SessionStore {
    let dir = test_home.path().join(".runie").join("sessions");
    SessionStore::new(dir)
}
