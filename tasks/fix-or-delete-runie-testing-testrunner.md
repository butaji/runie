# Fix or delete runie-testing TestRunner

## Status

`done`

## Context

`crates/runie-testing/src/runner.rs:20-114` provided `TestRunner::submit` and `expect_event` but was unused outside its own tests. It polled with `tokio::time::sleep(Duration::from_millis(10))`.

## Goal

Either adopt `TestRunner` in agent integration tests (replace polling with `tokio::sync::Notify`/channel) or delete it.

## Decision

**Delete.** `runner.rs` was never adopted; it existed only in the task's initial scope draft.

## Acceptance Criteria

- [x] Decide adopt or delete. — **Delete** chosen; runner.rs deleted.
- [x] If adopt, remove `sleep` polling; use deterministic notification. — N/A
- [x] If delete, remove file and any references. — `crates/runie-testing/src/runner.rs` does not exist.
- [x] All tests pass.

## Design Impact

No change to TUI element design or composition. Only test infrastructure changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All tests pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A.

## Verification

- `crates/runie-testing/src/runner.rs` does not exist (verified 2026-07-01).
- `cargo check --workspace` passes.
- `cargo test --workspace` passes (2806+ tests).
