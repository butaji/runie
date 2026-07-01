# Fix or delete runie-testing TestRunner

## Status

`done`

## Context

`crates/runie-testing/src/runner.rs:20-114` provides `TestRunner::submit` and `expect_event` but is unused outside its own tests. It polls with `tokio::time::sleep(Duration::from_millis(10))`.

## Goal

Either adopt `TestRunner` in agent integration tests (replace polling with `tokio::sync::Notify`/channel) or delete it.

## Acceptance Criteria

- [ ] Decide adopt or delete.
- [ ] If adopt, remove `sleep` polling; use deterministic notification.
- [ ] If delete, remove file and any references.
- [ ] All tests pass.

## Design Impact

No change to TUI element design or composition. Only test infrastructure changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
