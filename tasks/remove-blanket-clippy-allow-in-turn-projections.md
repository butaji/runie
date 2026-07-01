# Remove blanket clippy allow in turn projections

## Status

`todo`

## Context

`crates/runie-core/src/model/state/turn_projections.rs:1` has `#![allow(clippy::all)]`, hiding real quality issues in production code.

## Goal

Remove the module-level allow and fix or narrowly suppress only unavoidable lints.

## Acceptance Criteria
- [ ] Remove `#![allow(clippy::all)]`.
- [ ] Fix or individually allow remaining lints.
- [ ] `cargo clippy --workspace -- -D warnings` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Clippy clean.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
