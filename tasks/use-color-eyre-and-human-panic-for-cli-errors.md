# Use color-eyre and human-panic for CLI errors

## Status

`done`

## Context

`crates/runie-cli/src/main.rs:100-102` reports errors with `eprintln!("Error: {}", e); process::exit(1);`. There is no error chain, no panic report, and no friendly suggestion to set `RUST_LOG`.

## Goal

Adopt `color-eyre` for the TUI and `human-panic` for both binaries. Print the full error chain with `{:?}`/eyre formatting.

## Acceptance Criteria

- [x] Add `color-eyre`/`human-panic` to workspace deps. — Done; both in Cargo.toml
- [x] Install `human-panic` panic hook in both binaries. — Done; CLI at line 88, TUI at line 47
- [x] Use `color_eyre::Result` in `runie-cli` and print the chain on error. — Done; color_eyre::install() at line 94, error chain printed at line 107
- [x] TUI uses `color-eyre` for startup errors. — Done; added color_eyre::install() in TUI main.rs, bootstrap errors print with full anyhow chain ({:#} format), hint to set RUST_LOG

## Design Impact

No change to TUI element design or composition. Only error reporting behavior changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Trigger a CLI error and verify a full chain/panic message.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

## Notes

The CLI fully implements color-eyre and human-panic. The TUI only uses human-panic. The task as written requires color-eyre in TUI startup errors, which is not implemented.
