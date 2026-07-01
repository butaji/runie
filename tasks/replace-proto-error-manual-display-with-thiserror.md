# Replace proto error manual Display with thiserror

## Status

`todo`

## Context

`crates/runie-core/src/proto/error.rs:48-54` hand-implements `fmt::Display` and `std::error::Error` for a JSON-RPC-style error struct.

## Goal

Use `#[derive(Debug, Error)]` from `thiserror`.

## Acceptance Criteria
- [ ] Derive with `thiserror`.
- [ ] Preserve source chain behavior.
- [ ] Delete manual impls.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for Display and source chain.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Protocol tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
