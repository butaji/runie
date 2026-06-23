//! Tests for runie-agent
//! Note: Some tests were archived due to dependencies on unbuilt crates.

mod events;
mod minimax_like;
mod parser;
mod permissions;
mod safety;
mod tools;
mod tool_marker_state;
mod turn;
mod turn_gate;

use std::sync::Once;

/// Enable mock provider for all tests. Without this, the "mock" provider key is
/// not registered in the provider registry, causing DynProvider::new("mock", ...)
/// to return UnknownProvider error.
static ENABLE_MOCK: Once = Once::new();

/// Call this at the start of each test that uses the "mock" provider.
pub(crate) fn ensure_mock_provider() {
    ENABLE_MOCK.call_once(|| {
        runie_core::provider_registry::set_mock_enabled(true);
    });
}
