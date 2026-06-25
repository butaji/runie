# Flatten update/login_flow/tests.rs directory

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: consolidate-dual-path-modules
**Blocks**: none

## Description

`crates/runie-core/src/update/login_flow/` is a directory containing a single file `tests.rs` (45 LOC) — the tests for the sibling `update/login_flow.rs`. This is the dual-path pattern (`foo.rs` + `foo/` dir) scoped to `update/`, but the directory holds only tests, not submodules, so it's even less justified. The single-file directory exists only because `foo.rs` + `foo/tests.rs` is disallowed by Rust 2018.

After `consolidate-dual-path-modules` converts `login_flow.rs` → `login_flow/mod.rs`, this directory naturally collapses: `login_flow/tests.rs` sits next to `login_flow/mod.rs` and the separate `update/login_flow/` dir is gone. This task is the `update/`-scoped follow-up.

The same pattern exists at `update/dialog/panel.rs` + `update/dialog/panel/tests.rs` and `update/dialog/form.rs` + `update/dialog/form_tests.rs` — both are handled by `consolidate-dual-path-modules` and `relocate-loose-tests-files` respectively. This task exists only for the `update/login_flow/` case because it's a single-file directory (more egregious than the others).

## Acceptance Criteria

- [ ] `crates/runie-core/src/update/login_flow.rs` → `update/login_flow/mod.rs` (via `consolidate-dual-path-modules` or this task).
- [ ] `crates/runie-core/src/update/login_flow/tests.rs` content moves into `update/login_flow_tests.rs` (flat sibling) OR into `update/login_flow/tests.rs` (inside the converted module dir). Pick one and apply consistently with `relocate-loose-tests-files`.
- [ ] The empty `update/login_flow/` directory (if flat-file option chosen) is removed.
- [ ] `update/mod.rs` `pub(crate) mod login_flow;` declaration still resolves.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds (the 2 tests in `tests.rs` stay green).

## Tests

### Layer 1 — State/Logic
- [ ] `provider_base_url_tests_still_pass` — the 2 existing tests (`provider_base_url_uses_registry_default_for_new_provider`, `provider_base_url_preserves_saved_custom_url`) stay green after the move.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_login_flow_module_resolves` — `cargo check -p runie-core` green after the flatten.

## Files touched

- `crates/runie-core/src/update/login_flow.rs` → `update/login_flow/mod.rs` (or stays as `login_flow.rs` with sibling `login_flow_tests.rs`)
- `crates/runie-core/src/update/login_flow/tests.rs` (move)
- `crates/runie-core/src/update/login_flow/` (remove dir if flattened)
- `crates/runie-core/src/update/mod.rs` (update `mod` declaration if needed)

## Notes

Depends on `consolidate-dual-path-modules` to set the workspace-wide convention first. Trivial; group with `relocate-loose-tests-files` in the same commit if convenient. The `update/dialog/panel/` and `update/dialog/form_tests.rs` cases are owned by those other tasks and not duplicated here.
