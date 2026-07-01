# Extract runie-test-utils crate

## Status

`todo`

## Context

`runie-core/src/tests/support.rs` and `runie-testing/src/tests/state.rs` define overlapping `fresh_state`, `type_str`, `exec` helpers.

## Goal

Create a `runie-test-utils` crate that both can depend on; delete duplicates.

## Acceptance Criteria
- [ ] Create `crates/runie-test-utils/`.
- [ ] Move shared helpers; keep crate-local ones where needed.
- [ ] Update all imports and tests.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo test --workspace` passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
