# Remove unused direct deps from runie-tui

## Status

`done`

## Context

`crates/runie-tui/Cargo.toml:37-39` declares `tui-textarea`, `ractor`, and `tracing-subscriber` but none are referenced in `crates/runie-tui/src/**/*.rs`.

## Goal

Delete the three unused direct dependencies from `runie-tui/Cargo.toml`.

## Acceptance Criteria
- [x] Delete `tui-textarea`, `ractor`, and `tracing-subscriber` lines.
- [x] `cargo check -p runie-tui` passes.
- [x] No source references remain.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A (dependency-only change).
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check -p runie-tui` and `cargo test -p runie-tui` pass.
- **Live tmux testing session (required):** Launch TUI and verify it starts.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
