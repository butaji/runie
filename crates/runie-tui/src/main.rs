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

use tokio::runtime::Builder;

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
    /// Show the onboarding flow with the mock provider available.
    #[arg(long)]
    mock_onboarding: bool,
    /// Mock model/fixture to use with --mock or --mock-onboarding (e.g. echo, list_dir, read_file).
    #[arg(long)]
    mock_model: Option<String>,
}

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::DisableFocusChange,
            // Defensive release: restores native terminal selection if mouse
            // capture was ever enabled (crash, older runie version).
            crossterm::event::DisableMouseCapture,
            crossterm::terminal::LeaveAlternateScreen,
        );
        let _ = runie_tui::terminal_setup::reset_keyboard_enhancements(&mut std::io::stdout());
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    // Install human-panic hook for crash reports.
    human_panic::setup_panic!();

    // Install color-eyre for better error chains.
    let _ = color_eyre::install();

    // Initialize tracing for TUI mode: JSON file logging + compact error console.
    tracing_init::init_tui();

    // One-time migration: move any legacy plaintext ~/.runie/auth.json into
    // the OS keyring and rename it to auth.json.bak. Best-effort — never abort
    // startup if the keyring is unavailable (headless/CI).
    if let Err(e) = runie_core::auth::migrate_legacy_auth() {
        tracing::warn!("legacy auth migration skipped: {e}");
    }

    let cli = Cli::parse();
    enable_mock_if_requested(cli.mock, cli.mock_onboarding, cli.mock_model.as_deref());
    if cli.dry_run || cli.preview {
        let report = runie_core::run_dry_run(&runie_core::Config::load(None));
        println!("{report}");
        return Ok(());
    }

    let _cleanup = Cleanup;

    // Build a multi-threaded tokio runtime manually so we can explicitly shut
    // it down after the TUI exits.  Without this, the `#[tokio::main]` implicit
    // runtime parks its worker threads indefinitely after `main()` returns, and
    // the process hangs around for 15+ seconds (or forever on macOS) because the
    // worker threads never release the process.
    let rt = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");

    // Build the TUI runtime with production settings.
    let tui_runtime = TuiRuntime::builder()
        .backend(BackendType::Crossterm)
        .build();

    // Run the TUI on the tokio runtime.  The `block_on` call returns once the
    // TUI's `run()` future completes (i.e. when the user quits).
    rt.block_on(async {
        let _ = tui_runtime.run().await;
    });

    // Drop the tracing file guard so the non-blocking worker thread stops.
    runie_core::tracing_init::shutdown();

    // Explicitly shut down the tokio runtime's worker threads.  This is the key
    // fix: without it the runtime parks its threads indefinitely after `main()`
    // returns and the process stays alive for 15+ seconds.
    rt.shutdown_background();

    // Exit immediately — no further cleanup needed.
    std::process::exit(0);
}
