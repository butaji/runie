# Deduplicate shared test helpers (fresh_state + siblings)

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Tests are ~63% of `runie-core` LOC (14,539 in `tests/` + ~18,853 inline `#[cfg(test)]`). A handful of byte-identical helpers repeat across that mass: `fresh_state()` (20 sites — literally `AppState::default()`), `type_str(state, text)` (5 sites), `exec(state, text)` (sets input + submits, multiple sites), `tmp_store()` (2 sites — temp SessionStore), `minimal_session(name)` (1 site but reusable), and `ENV_LOCK` / `write_skill` helpers. Each duplicate is drift-prone and fragments the test-support surface. Promote all of them into `runie-testing` (or a `#[cfg(test)] mod support`), then replace every local copy.

Pareto framing: this single task touches the 80% of the codebase (tests) that holds the duplication, with a 20%-effort mechanical sweep.

## Acceptance Criteria

- [ ] A single shared test-support module exposes: `fresh_state()`, `type_str(state, text)`, `exec(state, text)`, `tmp_store()`, `minimal_session(name)`, `ENV_LOCK`, `write_skill(dir, name, desc, ctx)`.
- [ ] All local `fresh_state` copies (20 sites) replaced with the shared import.
- [ ] All local `type_str` copies (5 sites) replaced.
- [ ] All local `exec` / `tmp_store` / `minimal_session` / `write_skill` copies replaced.
- [ ] `rg -c "fn fresh_state" crates/` returns exactly 1 (the shared definition).
- [ ] `rg -c "fn type_str" crates/` returns exactly 1.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `shared_fresh_state_is_default` — `fresh_state() == AppState::default()`.
- [ ] `shared_type_str_appends` — `type_str` produces the expected input buffer content + cursor.
- [ ] `shared_exec_submits_command` — `exec(state, "/save")` triggers submit.
- [ ] `shared_tmp_store_is_unique` — two `tmp_store()` calls return non-overlapping dirs.
- [ ] `shared_minimal_session_has_defaults` — `minimal_session("x")` has expected fields.

### Layer 2 — Event Handling
- N/A — test helpers only.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- N/A.

## Files touched

- `crates/runie-testing/src/lib.rs` (or new `crates/runie-core/src/tests/support.rs`) — shared definitions
- `crates/runie-core/src/tests/slash.rs` — drop local `fresh_state`/`type_str`/`exec`/`tmp_store`/`minimal_session` (current de-facto shared source)
- `crates/runie-core/src/tests/safety.rs` — drop local `fresh_state`/`type_str`
- ~20 test files under `crates/runie-core/src/tests/` and `crates/runie-tui/src/tests/core/` — replace local helpers with import
- `crates/runie-core/src/skill/mod.rs` — move `write_skill` test helper to shared module

## Notes

Keep `fresh_state` trivially `AppState::default()` so semantics don't diverge. The `slash.rs` file currently acts as a de-facto shared helper module (other tests import `slash::fresh_state`) — formalize this by moving the helpers out of `slash.rs` into the dedicated support module, then `slash.rs` imports them too. `ENV_LOCK` serializes tests that touch env vars — keep it in the shared module with a clear comment. This task is the 80/20 of test cleanup: one module, ~30 duplicate-deletions, zero behavior change.
