# Deduplicate fresh_state test helper

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`fn fresh_state() -> AppState` is duplicated ~38× across test files (20+ in `runie-core/src/tests/`, 14+ in `runie-tui/src/tests/core/`), most literally `AppState::default()`. Shared `pub fn fresh_state` versions already exist in `slash.rs`/`safety.rs` but siblings don't reuse them. Companion `fn type_str(state, text)` is byte-identical in `copy.rs`, `slash.rs`, `safety.rs`.

## Acceptance Criteria

- [ ] A single shared test-support module exposes `pub fn fresh_state()` and `pub fn type_str(state, text)` (extend `runie-testing` or a `#[cfg(test)] mod support`).
- [ ] All ~38 local `fresh_state` copies replaced with the shared import.
- [ ] All `type_str` copies replaced with the shared import.
- [ ] `rg -c "fn fresh_state" crates/` returns exactly 1 (the shared definition).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `shared_fresh_state_is_default` — `fresh_state() == AppState::default()`.
- [ ] `shared_type_str_appends` — `type_str` produces the expected input buffer content.

### Layer 2 — Event Handling
- N/A — test helper only.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- N/A.

## Files touched

- `crates/runie-testing/src/lib.rs` (or new `crates/runie-core/src/tests/support.rs`)
- ~38 test files under `crates/runie-core/src/tests/` and `crates/runie-tui/src/tests/core/`

## Notes

Drift-prone duplication. Keep the shared helper trivially `AppState::default()` so semantics don't diverge. Add `type_str` next to it.
