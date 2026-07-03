//! Runie Terminal — Non-blocking event loop with render actor
//!
//! Architecture (impossible to block by design):
//!   1. Event loop: single-threaded async, only async ops
//!   2. State: owned by event loop, mutable borrow per event
//!   3. Snapshot: immutable frame description (the UI DSL)
//!   4. Render actor: owns Terminal, receives Snapshots via channel
//!   5. If render is slow, old Snapshots are dropped — event loop never waits
//!
//! Event Bus Integration:
//!   - EventBus<Event> for cross-component communication
//!   - SessionActor subscribes to bus, persists durable events to JSONL

use clap::Parser;
use runie_core::tracing_init;
use runie_tui::mock_cmd::enable_mock_if_requested;
use std::io;

use runie_tui::bootstrap::{BackendType, TuiRuntime};

/// Runie TUI CLI arguments.
#[derive(Parser, Debug)]
#[command(name = "runie-tui", version)]
struct Cli {
    /// Show dry-run preview without starting the TUI.
    #[arg(long)]
    dry_run: bool,
    /// Alias for --dry-run (preview mode).
    #[arg(long, hide = true)]
    preview: bool,
    /// Enable the mock provider (no API key required).
    #[arg(long)]
    mock: bool,
    /// Mock model/fixture to use with --mock (e.g. echo, list_dir, read_file).
    #[arg(long, requires = "mock")]
    mock_model: Option<String>,
}

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::DisableFocusChange,
            crossterm::terminal::LeaveAlternateScreen,
        );
        let _ = runie_tui::terminal_setup::reset_keyboard_enhancements(&mut std::io::stdout());
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> io::Result<()> {
    // Install human-panic hook for crash reports.
    human_panic::setup_panic!();

    // Install color-eyre for better error chains.
    let _ = color_eyre::install();

    // Initialize tracing for TUI mode: JSON file logging + compact error console.
    tracing_init::init_tui();

    let cli = Cli::parse();
    enable_mock_if_requested(cli.mock, cli.mock_model.as_deref());
    if cli.dry_run || cli.preview {
        let report = runie_core::run_dry_run(&runie_core::Config::load(None));
        println!("{report}");
        return Ok(());
    }

    let _cleanup = Cleanup;

    // Build and run the TUI runtime with production settings.
    let runtime = TuiRuntime::builder()
        .backend(BackendType::Crossterm)
        .build();

    runtime.run().await
}
