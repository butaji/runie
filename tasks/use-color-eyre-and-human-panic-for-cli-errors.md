# Use color-eyre and human-panic for CLI errors

## Status

`todo`

## Context

`crates/runie-cli/src/main.rs:100-102` reports errors with `eprintln!("Error: {}", e); process::exit(1);`. There is no error chain, no panic report, and no friendly suggestion to set `RUST_LOG`.

## Goal

Adopt `color-eyre` for the TUI and `human-panic` for both binaries. Print the full error chain with `{:?}`/eyre formatting.

## Acceptance Criteria

- [ ] Add `color-eyre`/`human-panic` to workspace deps.
- [ ] Install `human-panic` panic hook in both binaries.
- [ ] Use `color_eyre::Result` in `runie-cli` and print the chain on error.
- [ ] TUI uses `color-eyre` for startup errors.

## Design Impact

No change to TUI element design or composition. Only error reporting behavior changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Trigger a CLI error and verify a full chain/panic message.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
