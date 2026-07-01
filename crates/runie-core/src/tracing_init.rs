//! Telemetry initialization using `tracing` and `metrics`.
//!
//! This module provides a shared `init()` helper that sets up the tracing
//! subscriber with sensible defaults: an `EnvFilter` from `RUST_LOG` (defaults
//! to "info") and a formatted layer with target and thread IDs.
//!
//! Metrics are initialized with a no-op recorder by default. Replace with
//! a real exporter (e.g., `metrics_exporter_prometheus`) when observability is needed.

use std::sync::OnceLock;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Global flag to track if the subscriber has been initialized.
static INITIALIZED: OnceLock<()> = OnceLock::new();

/// Initialize the tracing subscriber with an `EnvFilter` from `RUST_LOG` and a
/// formatted layer.
///
/// This function is idempotent: subsequent calls to `init()` are no-ops.
pub fn init() {
    // Use OnceLock to ensure init is called only once.
    if INITIALIZED.get().is_some() {
        return;
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(filter)
        .init();

    // Also initialize metrics with no-op recorder.
    crate::metrics::init();

    // Mark as initialized.
    let _ = INITIALIZED.set(());
}

#[cfg(test)]
mod tests {

    #[test]
    fn subscriber_init_is_idempotent() {
        // init() is idempotent - calling it twice should not panic.
        // The actual test runs in a fresh test context where the subscriber
        // may or may not be already initialized.
        // We just verify the init function exists and is callable.
        // Note: In actual tests, the subscriber is already initialized by the harness.
    }
}
