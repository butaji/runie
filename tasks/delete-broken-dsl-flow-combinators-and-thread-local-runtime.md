# Delete broken DSL flow combinators and thread-local runtime

## Status

`done`

## Context

`crates/runie-core/src/dsl/flow.rs:169-214` exposes `.map`, `.filter`, and `.branch` combinators that ignore their closures; `runtime.rs` uses a thread-local `CURRENT_RUNTIME` global and the `broadcast_fact`/`notify` methods are `TODO` no-ops. The DSL adds ~800 LOC with no working behavior.

## Goal

Delete the DSL veneer (`flow.rs`, `runtime.rs`, `examples.rs`) and call plain Rust functions / match directly from command/update handlers. Pass any needed runtime context explicitly.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/dsl/flow.rs`, `runtime.rs`, `examples.rs`.
- [ ] Update callers in command/update handlers to use plain Rust.
- [ ] No regressions in declarative command execution.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition. Only internal DSL implementation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for the equivalent command/update logic.
- **Layer 2 — Event Handling:** Command messages produce the same events.
- **Layer 3 — Rendering:** `TestBackend` snapshots match.
- **Layer 4 — E2E:** Headless CLI slash commands work.
- **Live tmux validation:** Common slash commands behave as before.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
