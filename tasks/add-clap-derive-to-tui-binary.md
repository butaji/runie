# Add clap derive to TUI binary

## Status

`done`

**Completed:** 2026-07-01

## Context

`crates/runie-tui/src/main.rs` parsed `std::env::args()` by hand and `dry_run_cmd.rs` scanned for `--dry-run`/`--preview`; unknown flags were silently ignored.

## What was done

1. Added `clap.workspace = true` to `crates/runie-tui/Cargo.toml`
2. Added a `Cli` struct with `#[derive(Parser, Debug)]`:
   ```rust
   #[derive(Parser, Debug)]
   #[command(name = "runie", version)]
   struct Cli {
       /// Show dry-run preview without starting the TUI.
       #[arg(long)]
       dry_run: bool,
       /// Alias for --dry-run (preview mode).
       #[arg(long, hide = true)]
       preview: bool,
   }
   ```
3. Replaced manual args scanning with `Cli::parse()` in `main()`
4. Simplified `dry_run_cmd.rs` to `run_dry_run_if_requested(dry: bool)` — removed the manual args scanning
5. Updated tests to use the new function signature

## Acceptance Criteria

- [x] Define `Cli` derive struct. — **Done**
- [x] Replace manual scanning. — **Done**
- [x] Update callers/scripts if needed. — **Done** (dry_run_cmd.rs simplified)

## Tests

- `cargo check -p runie-tui` passes
- `cargo test -p runie-tui` passes
- `cargo run --bin runie-tui -- --help` shows proper help
- `cargo run --bin runie-tui -- --version` shows version
- `cargo run --bin runie-tui -- --dry-run` works (runs dry-run preview)
- `cargo clippy -p runie-tui` has no new warnings
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
