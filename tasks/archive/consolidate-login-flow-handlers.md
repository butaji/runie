# Consolidate login flow handlers into login_flow module

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: none
**Blocks**: audit-borrow-workarounds, consolidate-login-logout-tests

## Description

Login flow handlers were consolidated into `crates/runie-core/src/login_flow/` module. The directory now contains:
- `handlers.rs` - event handlers for the login flow
- `state.rs` - login flow state machine
- `panels.rs` - UI panel builders
- `panel_ops.rs` - panel operations
- `validation.rs` - API key validation
- `e2e_tests.rs` - end-to-end tests
- `handlers_tests.rs` - handler unit tests
- `state_tests.rs` - state machine tests

## Evidence of Completion

```bash
$ ls crates/runie-core/src/login_flow/
e2e_tests.rs
handlers_tests.rs
handlers.rs
mod.rs
panel_ops.rs
panels.rs
state_tests.rs
state.rs
tests.rs
validation.rs
```

The `update/login_flow.rs` handler was moved to `login_flow/handlers.rs` and the old location removed.

## Notes

This task was completed as part of the R4 refactoring but the task file was created retroactively when the dependency chain was analyzed.
