# Fix Broken Symbol References After Login Flow Refactor

**Status**: todo
**Milestone**: MVP
**Category**: Core Architecture
**Priority**: P0
**Depends on**: resolve-merge-conflicts

## Description

The login flow was refactored but the rename from `build_login_root` to
`build_login_stack` was only partial. Three call sites reference
`build_login_stack` which does not exist; the function in
`crates/runie-core/src/login_flow.rs:200` is `build_login_root()`. There is
also a docstring example in `commands/dsl/builder.rs` with a stale
signature.

## Acceptance Criteria

- [ ] `use crate::login_flow::build_login_stack;` in `crates/runie-core/src/update/login_flow.rs:125` is replaced with `use crate::login_flow::build_login_root;` (or a new function is added — see Notes)
- [ ] `use crate::login_flow::{…, build_login_stack, …}` in `crates/runie-core/src/login_flow/tests/state.rs:5` is corrected
- [ ] `build_login_stack(...)` calls in `login_flow/tests/state.rs:311,318,322` are corrected
- [ ] `cargo build --workspace` produces no `cannot find function build_login_stack` errors
- [ ] The docstring example in `commands/dsl/builder.rs:102` (`.panel(|state, _| build_login_root(state))`) is fixed — `build_login_root()` takes no arguments; the closure form should be `.panel(|_, _| build_login_root())` or the function should be changed to take `&AppState` if that's the intended signature

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace --tests` succeeds
- [ ] `cargo test -p runie-core --lib login_flow::tests::state` passes (this test file currently fails to compile)
- [ ] `cargo test --doc -p runie-core commands::dsl` passes (the docstring example)

### Layer 2 — Event Handling
- [ ] `cargo test -p runie-core --lib update::login_flow::tests` passes (login flow event dispatch uses `build_login_root` indirectly via `rebuild_login_dialog`)

## Notes

**Two valid resolution paths** — pick one and document the choice in the commit:

1. **Rename the function** to `build_login_stack` and update `login_flow.rs:200` plus the `lib.rs:74` re-export. This matches the new "stack" terminology used in the call sites.

2. **Keep the function** named `build_login_root` and rename the call sites. This is the smaller diff.

**Recommendation:** Path 2. The function returns a `PanelStack` whose *root* panel is the provider picker; `build_login_root` is the more descriptive name. Only three call sites need updating.

**Out of scope:**
- Unifying login-flow handlers across `login_flow.rs`, `update/login_flow.rs`, and the in-`update/mod.rs` merge survivors (see `deduplicate-login-flow`)
- Adding a `build_login_stack` variant that takes an arbitrary step (would require a real state machine, not just a renamed function)

**Verification:**
```bash
cargo build --workspace 2>&1 | grep -i 'build_login_stack' && echo "FAIL" || echo "OK"
cargo test -p runie-core --lib login_flow::tests::state
```
