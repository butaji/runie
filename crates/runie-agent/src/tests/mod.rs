//! Tests for runie-agent
//! Note: Some tests were archived due to dependencies on unbuilt crates.

mod events;
mod minimax_like;
mod parser;
mod permissions;
mod safety;
mod tool_marker_state;
mod tools;
mod turn;
mod turn_gate;

use tokio::sync::Mutex;

/// Serializes tests that mutate the global mock-enabled state.
pub(crate) static MOCK_STATE_LOCK: Mutex<()> = Mutex::const_new(());

/// Guard that holds the mock-state lock for the duration of a test and restores
/// the previous mock state on drop.
pub(crate) struct MockGuard {
    prev: bool,
    #[allow(dead_code)]
    guard: tokio::sync::MutexGuard<'static, ()>,
}

impl Drop for MockGuard {
    fn drop(&mut self) {
        runie_core::provider::set_mock_enabled(self.prev);
    }
}

/// Enable mock provider for all tests. Without this, the "mock" provider key is
/// not registered in the provider registry, causing DynProvider::new("mock", ...)
/// to return UnknownProvider error.
pub(crate) async fn ensure_mock_provider() -> MockGuard {
    let guard = MOCK_STATE_LOCK.lock().await;
    let prev = runie_core::provider::is_mock_enabled();
    runie_core::provider::set_mock_enabled(true);
    MockGuard { prev, guard }
}
