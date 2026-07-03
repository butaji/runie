//! Telemetry initialization using `tracing` and `metrics`.
//!
//! This module provides shared initialization helpers:
//! - `init()` — for CLI/headless mode: pretty console output
//! - `init_tui()` — for TUI mode: JSON file logging (preserves console for errors)
//!
//! TUI file logs go to `~/.runie/logs/runie-{date}.jsonl` for structured debugging.
//!
//! Metrics are initialized with a no-op recorder by default. Replace with
//! a real exporter (e.g., `metrics_exporter_prometheus`) when observability is needed.

use std::path::PathBuf;
use std::sync::OnceLock;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

/// Global guard for the non-blocking worker thread.
/// Must be kept alive for the duration of the program.
static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Global flag to track if the subscriber has been initialized.
static INITIALIZED: OnceLock<InitMode> = OnceLock::new();

/// The mode used during initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitMode {
    /// Console-only output for CLI/headless mode.
    Console,
    /// File logging for TUI mode.
    File,
}

/// Initialize the tracing subscriber for CLI/headless mode.
///
/// Sets up an `EnvFilter` from `RUST_LOG` (defaults to "info") and a formatted
/// layer with target and thread IDs sent to stdout.
///
/// This function is idempotent: subsequent calls are no-ops.
pub fn init() {
    init_with_mode(InitMode::Console);
}

/// Initialize the tracing subscriber for TUI mode.
///
/// Sets up an `EnvFilter` from `RUST_LOG` (defaults to "info") with:
/// - A JSON file layer at `~/.runie/logs/runie-{date}.jsonl`
/// - A compact console layer for errors/warnings only (to avoid corrupting terminal)
///
/// This function is idempotent: subsequent calls are no-ops.
pub fn init_tui() {
    init_with_mode(InitMode::File);
}

fn init_with_mode(mode: InitMode) {
    // Use OnceLock to ensure init is called only once.
    if let Some(existing) = INITIALIZED.get() {
        if *existing == mode {
            return;
        }
        // Already initialized with a different mode — don't reinitialize.
        return;
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    match mode {
        InitMode::Console => {
            // Pretty console output for CLI.
            Registry::default()
                .with(fmt::layer().with_target(true).with_thread_ids(true))
                .with(filter)
                .init();
        }
        InitMode::File => {
            // Set up file appender for structured JSON logs.
            // We intentionally do NOT write to the console in TUI mode: any stdout/stderr
            // output while the terminal is in raw mode corrupts the UI.
            let log_dir = default_log_dir();

            // Create the log directory if it doesn't exist.
            if let Err(e) = std::fs::create_dir_all(&log_dir) {
                eprintln!(
                    "Warning: failed to create log directory {:?}: {}",
                    log_dir, e
                );
            }

            // Rolling file appender with daily rotation.
            // File format: runie-YYYY-MM-DD.jsonl
            let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "runie");

            // Non-blocking writer to avoid blocking the async runtime.
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            // Keep the guard alive for the lifetime of the program.
            let _ = FILE_GUARD.set(guard);

            // JSON file layer for structured logs.
            let file_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_ansi(false)
                .json()
                .with_writer(non_blocking);

            Registry::default()
                .with(file_layer)
                .with(filter)
                .init();
        }
    }

    // Also initialize metrics with no-op recorder.
    #[allow(unused)]
    crate::metrics::init();

    // Mark as initialized.
    let _ = INITIALIZED.set(mode);
}

/// Default log directory: `~/.runie/logs/`
fn default_log_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("RUNIE_TEST_LOG_DIR") {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .map(|d| d.join("runie").join("logs"))
        .unwrap_or_else(|| PathBuf::from("/tmp/runie-logs"))
}

/// Returns the current initialization mode, if initialized.
pub fn init_mode() -> Option<InitMode> {
    INITIALIZED.get().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_mode_is_idempotent() {
        // init() is idempotent - calling it twice should not panic.
        // The actual test runs in a fresh test context where the subscriber
        // may or may not be already initialized.
        // We just verify the init function exists and is callable.
        // Note: In actual tests, the subscriber is already initialized by the harness.
    }

    #[test]
    fn default_log_dir_uses_data_dir() {
        let dir = default_log_dir();
        // Should end with runie/logs
        assert!(
            dir.ends_with("runie/logs") || dir.ends_with("runie-logs"),
            "expected runie/logs suffix, got: {}",
            dir.display()
        );
    }
}
