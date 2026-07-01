# Delete dead runie-core testing module

## Status

`done`

## Context

`crates/runie-core/src/testing/mod.rs` and `crates/runie-core/src/testing/actor_harness.rs` were not included in `lib.rs` and contained a non-compiling `CounterActor` example. The module was dead code.

## Goal

Delete the module and any references. Tests that need a bus can use `tokio::sync::broadcast` directly; actor tests can use ractor utilities or `tokio::sync` channels.

## Acceptance Criteria

- [x] Delete `crates/runie-core/src/testing/`. — Done; directory does not exist.
- [x] Ensure `cargo check --workspace` still passes. — Verified.
- [x] No test or doc references remain. — Verified with grep; no `::testing::` references.

## Design Impact

No change to TUI element design or composition. Only dead code removal.

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
- [x] **Live tmux run tests** — N/A.

## Verification

- `crates/runie-core/src/testing/` does not exist (verified 2026-07-01).
- No `::testing::` references found in `crates/runie-core/`.
- `cargo check --workspace` passes.
- `cargo test --workspace` passes (2806+ tests).
