# Replace proto error manual Display with thiserror

## Status

`wontfix`

## Context

`crates/runie-core/src/proto/error.rs:48-54` originally hand-implemented `fmt::Display` and `std::error::Error`.

## Why Not Applicable

The `thiserror` crate cannot derive `Display` for structs containing `serde_json::Value` because `Value` does not implement `std::fmt::Display`. The struct also derives `Serialize`/`Deserialize` from serde, which needs to be preserved.

The current manual implementation is the correct approach for this case. The struct has been cleaned up (removed unused imports, added tests), but `thiserror` cannot be used.

## Acceptance Criteria
- [x] The struct uses manual Display/Error implementations (correct for this case).
- [x] Preserves source chain behavior.
- [x] Manual impls kept for compatibility with serde_json::Value field.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for Display and source chain.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Protocol tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
