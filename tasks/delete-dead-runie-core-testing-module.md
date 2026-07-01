# Delete dead runie-core testing module

## Status

`done`

Note: Module doesn't exist - was either never created or already deleted.

## Context

`crates/runie-core/src/testing/mod.rs` and `crates/runie-core/src/testing/actor_harness.rs` are not included in `lib.rs` and contain a non-compiling `CounterActor` example. The module is dead code.

## Goal

Delete the module and any references. Tests that need a bus can use `tokio::sync::broadcast` directly; actor tests can use ractor utilities or `tokio::sync` channels.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/testing/`.
- [ ] Ensure `cargo check --workspace` still passes.
- [ ] No test or doc references remain.

## Design Impact

No change to TUI element design or composition. Only dead code removal.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check --workspace` and `cargo test --workspace` pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
