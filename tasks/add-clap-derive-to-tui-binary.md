# Add clap derive to TUI binary

## Status

`todo`

## Context

`crates/runie-tui/src/main.rs` parses `std::env::args()` by hand and `dry_run_cmd.rs` scans for `--dry-run`/`--preview`; unknown flags are silently ignored.

## Goal

Add a `clap` derive `Cli` struct with explicit flags and get `--help`/`--version` for free.

## Acceptance Criteria
- [ ] Define `Cli` derive struct.
- [ ] Replace manual scanning.
- [ ] Update callers/scripts if needed.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** CLI help/version tests pass.
- **Live tmux validation:** `cargo run --bin runie -- --help` works.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
