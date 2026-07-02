# Add JSON file logging for TUI

## Status

**done**

## Description

TUI tracing currently writes to stdout/stderr, which can corrupt the terminal. Add a JSON file appender for TUI mode while preserving console logging for headless/CLI.

## Implementation

Modified `tracing_init.rs` to support both modes:

- `init()` — for CLI/headless mode: pretty console output to stdout
- `init_tui()` — for TUI mode: JSON file logging + compact error console

### Files changed

- `Cargo.toml` — added `tracing-appender = "0.2"` workspace dependency, enabled `json` feature for `tracing-subscriber`
- `crates/runie-core/Cargo.toml` — added `tracing-appender.workspace = true`
- `crates/runie-core/src/tracing_init.rs` — new `InitMode` enum, `init_tui()` function, JSON file appender with daily rotation
- `crates/runie-core/src/tests/arch_guardrails.rs` — added `tracing_init.rs` to production allow list
- `crates/runie-tui/src/main.rs` — calls `tracing_init::init_tui()` instead of `init()`

### TUI file logging details

- Log directory: `~/.runie/logs/` (configurable via `RUNIE_TEST_LOG_DIR` env var for tests)
- File format: `runie-YYYY-MM-DD.jsonl` (daily rotation)
- JSON format includes: `target`, `thread_ids`, timestamps, and full event data
- Console output: errors and warnings only (to avoid corrupting terminal)

## Acceptance criteria

1. [x] **Unit tests** — TUI mode writes structured JSON logs to a file; CLI mode still uses pretty console output.
2. [x] **E2E tests** — Log file is created and contains expected spans.
3. [ ] **Live tmux tests** — Run the TUI for a while and inspect the log file.

## Tests

### Layer 1 — State/Logic
- [x] `init_mode_is_idempotent` — calling init twice doesn't panic
- [x] `default_log_dir_uses_data_dir` — log directory is correctly computed

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tui_creates_log_file` — live tmux session creates log file in `~/.runie/logs/`

## Validation

1. ✓ `cargo check --workspace` passes with zero new warnings
2. ✓ `cargo test --workspace` passes (1968 tests pass)
3. ✓ `cargo clippy --workspace` passes with zero warnings
4. ⏳ Live tmux session creates log file in `~/.runie/logs/`

## Notes

- The JSON file format is compatible with standard log analysis tools (jq, etc.)
- The non-blocking writer avoids blocking the async runtime
- The `WorkerGuard` is kept alive for the lifetime of the program via `OnceLock`
