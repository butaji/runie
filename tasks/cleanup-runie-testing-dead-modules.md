# Cleanup runie-testing dead modules

## Status

`todo`

## Context

`runie-testing/src/timeout.rs` and `events.rs` are unused; `macros.rs` is misnamed.

## Goal

Delete or adopt dead modules; rename `macros.rs` to `conditional.rs`.

## Acceptance Criteria
- [ ] Delete `timeout.rs` and its tests.
- [ ] Delete or adopt `events.rs` builders.
- [ ] Rename `macros.rs` and update `lib.rs`.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo test -p runie-testing` passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
