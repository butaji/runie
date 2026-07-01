# Resolve runie-util micro-crate vs core re-exports

## Status

`done`

## Decision: Fold

Folded `runie-util` into `runie-core` because:
- The crate was tiny (2 modules, no complex external deps worth isolating)
- Both `runie-core` and `runie-tui` used these utilities
- Eliminates extra crate boundary without value

## Context

`runie-util` only exposes `display_width` and `labels`. `runie-core/src/display_width.rs` and `runie-core/src/labels.rs` are pure re-exports. The split adds crate boundaries without clear value.

## Goal

Either fold `runie-util` into `runie-core` and delete re-export stubs, or expand `runie-util` to own all generic helpers (display width, labels, which, bytesize, humantime, unicode-width).

## Acceptance Criteria

- [x] Decide fold vs expand.
- [x] Move code accordingly.
- [x] Delete duplicate re-exports.
- [x] All tests pass.

## Design Impact

No change to TUI element design or composition. Only crate structure changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check --workspace` and `cargo test --workspace` pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — N/A (crate structure change only).
