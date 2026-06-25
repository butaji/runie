# Deduplicate shared test helpers (fresh_state + siblings)

**Status**: done
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Tests have several duplicate helper definitions. After consolidation:

- `runie-testing/src/tests/state.rs` — canonical source for `fresh_state()`, `type_str()`, `exec()` shared by `runie-tui` tests.
- `runie-core/src/tests/support.rs` — canonical source for the same helpers for `runie-core` internal tests (including `populate_cache_from_login_config()`).
- `ENV_LOCK`, `tmp_store`, `minimal_session` live in `runie-core/src/tests/support.rs` only (they need `runie-core` internals).

Local copies removed from: `session_extra.rs`, `misc.rs`, `form_dialog.rs`.

> Note: `write_skill` does not exist in the codebase and was omitted.

## Acceptance Criteria

- [x] `runie-testing/src/tests/state.rs` exposes: `fresh_state()`, `type_str()`, `exec()`.
- [x] `runie-core/src/tests/support.rs` exposes: `fresh_state()`, `type_str()`, `exec()`, `tmp_store()`, `minimal_session()`, `ENV_LOCK`.
- [x] Local `exec` copies removed from `session_extra.rs`, `misc.rs`, `form_dialog.rs`.
- [x] `runie-tui` uses `runie_testing::exec` directly (no local wrapper).
- [x] `rg "^fn fresh_state|^pub fn fresh_state" crates/` returns 2 (one per crate: runie-testing + runie-core).
- [x] `rg "^fn type_str|^pub fn type_str" crates/` returns 2 (one per crate).
- [x] `rg "^fn exec\b|^pub fn exec\b" crates/` returns 2 (one per crate; runie-tui removed its local copy).
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `shared_fresh_state_is_default` — `fresh_state() == AppState::default()` (in `runie-testing/src/tests/state.rs`).
- [x] `shared_type_str_appends` — `type_str` produces the expected input buffer content (in `runie-testing`).
- [x] `shared_exec_submits_command` — `exec(state, "/save")` sets input and calls submit (in `runie-testing`).
- [x] `shared_tmp_store_is_unique` — two `tmp_store()` calls return non-overlapping dirs (in `support.rs`).
- [x] `shared_minimal_session_has_defaults` — `minimal_session("x")` has expected fields (in `support.rs`).

### Layer 2 — Event Handling
- N/A — test helpers only.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` green confirms all import paths resolved.

## Files touched

- `crates/runie-testing/src/tests/state.rs` — canonical helpers (new `tests/` subdirectory)
- `crates/runie-testing/src/tests/mod.rs` — module declaration
- `crates/runie-testing/src/lib.rs` — updated re-exports
- `crates/runie-core/src/tests/support.rs` — keep helpers, removed local copies from consumers
- `crates/runie-core/src/tests/session_extra.rs` — import `exec` from `crate::tests`; keep `Event` import
- `crates/runie-core/src/tests/misc.rs` — import `exec` from `crate::tests`
- `crates/runie-core/src/tests/form_dialog.rs` — import `tmp_store` from `crate::tests`
- `crates/runie-tui/src/tests/render/render_slash.rs` — use `runie_testing::exec` directly, removed local wrapper
