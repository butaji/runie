# Store and await HeadlessRuntime actor handles

## Status

`todo`

## Context

`crates/runie-core/src/headless_runtime.rs:41-44` drops config and provider actor join handles immediately after spawn.

## Goal

Store handles in `HeadlessRuntime` and expose an async `shutdown()` method.

## Acceptance Criteria
- [ ] Store actor join handles.
- [ ] Add `shutdown()` awaiting them.
- [ ] Update CLI/server callers.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless CLI tests still pass.
- **Live tmux validation:** Headless CLI exits cleanly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
