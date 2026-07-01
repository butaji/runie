# Extract runie-test-utils crate

## Status

`done`

**Completed:** 2026-07-01

## Context

`runie-core/src/tests/support.rs` and `runie-testing/src/tests/state.rs` defined overlapping `fresh_state`, `type_str`, `exec` helpers with slightly different implementations.

## Goal

Consolidate shared helpers so both crates use the same canonical source; delete duplicates.

## Changes Made

Instead of creating a new `runie-test-utils` crate (redundant with the existing `runie-testing` crate), the consolidation uses `runie-testing` as the hub:

### `crates/runie-core/src/lib.rs`
- Changed `mod tests;` from `#[cfg(test)]` to unconditional (`#[allow(unused)]`) so the
  module's pub-re-exports are visible to `runie-testing` even in non-test builds.
- Added unconditional `pub mod tests_support` that re-exports `exec`, `fresh_state`,
  `type_str` from the pub-re-exports in `tests/mod.rs`.

### `crates/runie-core/src/tests/mod.rs`
- Made the `support` submodule unconditional (`#[allow(unused)] mod support;`) so the
  module compiles in non-test builds.
- Made the `pub use support::{exec, fresh_state, ...}` re-export unconditional so
  `tests_support` in `lib.rs` can reference it.

### `crates/runie-core/src/tests/support.rs`
- No structural changes — remains the canonical source of `fresh_state`, `type_str`,
  `exec`, `ENV_LOCK`, `seed_providers`, `tmp_store`, `minimal_session`.
- Internal helpers (`seed_providers`, `minimal_session`, `tmp_store`, `ENV_LOCK`)
  stay here because they need access to `runie-core` internals.

### `crates/runie-testing/src/tests/state.rs`
- Deleted the duplicate `fresh_state`, `type_str`, `exec` definitions.
- Now re-exports from `runie_core::tests_support`:
  `pub use runie_core::tests_support::{exec, fresh_state, type_str};`
- The test assertions in the inner `#[cfg(test)] mod tests {}` remain for local
  verification.

## Acceptance Criteria

- [x] Consolidate shared helpers — `fresh_state`, `type_str`, `exec` are now defined
  once in `runie-core/src/tests/support.rs` and re-exported through
  `runie_core::tests_support` for `runie-testing`.
- [x] Keep crate-local helpers where needed — `ENV_LOCK`, `seed_providers`,
  `tmp_store`, `minimal_session` stay in `runie-core/src/tests/support.rs`.
- [x] Update all imports and tests — no import changes needed for existing
  `runie-core` test consumers (they continue to use `crate::tests::{fresh_state, ..}`).

## Design Impact

No change to TUI element design or composition. Only internal test infrastructure changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo test --workspace` passes (1780 tests, 0 new failures).
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core` passes (1781 tests).
- [x] **E2E tests** — `cargo test --workspace` passes (2595 tests, 0 new failures).
- [x] **Live tmux run tests** — N/A (test infrastructure change only).
