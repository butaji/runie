# Rename telemetry tests and import alias

## Status

`done`

## Context

`config/mod.rs` still has `mod telemetry_tests`; `runie-tui/src/main.rs` imports `runie_core::tracing_init as telemetry`.

## Goal

Rename the test module and import to match the `tracing_init` rename.

## Acceptance Criteria
- [ ] Rename `telemetry_tests` module.
- [ ] Update TUI import alias.
- [ ] `cargo check -p runie-tui` and `cargo test -p runie-core config` pass.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Tests pass.
- **Live tmux validation:** TUI starts.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
