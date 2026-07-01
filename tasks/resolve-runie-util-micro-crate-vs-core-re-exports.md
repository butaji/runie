# Resolve runie-util micro-crate vs core re-exports

## Status

`todo`

## Context

`runie-util` only exposes `display_width` and `labels`. `runie-core/src/display_width.rs` and `runie-core/src/labels.rs` are pure re-exports. The split adds crate boundaries without clear value.

## Goal

Either fold `runie-util` into `runie-core` and delete re-export stubs, or expand `runie-util` to own all generic helpers (display width, labels, which, bytesize, humantime, unicode-width).

## Acceptance Criteria

- [ ] Decide fold vs expand.
- [ ] Move code accordingly.
- [ ] Delete duplicate re-exports.
- [ ] All tests pass.

## Design Impact

No change to TUI element design or composition. Only crate structure changes.

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
