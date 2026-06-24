# Fix `PermissionGate` rename in `runie-testing`

**Status**: done
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-testing/src/fixtures.rs:6` imports a type that no longer exists:

```rust
use runie_core::permissions::{AutoAllowSink, PermissionManager, PermissionGate};
```

`PermissionGate` was renamed to `PermissionMode` in `runie-core::permissions`. The compiler reports:

```
error[E0432]: unresolved import `runie_core::permissions::PermissionGate`
help: a similar name exists in the module
  |
6 - use runie_core::permissions::{AutoAllowSink, PermissionManager, PermissionGate};
6 + use runie_core::permissions::{AutoAllowSink, PermissionManager, PermissionMode};
```

`PermissionGate` (the duplicated struct) still exists in `runie-agent/src/permission_gate.rs` — that one is the *real* gate used by the agent turn. Decide whether `fixtures.rs` wanted the agent gate or the core permission-mode enum and import the right one.

## Acceptance Criteria

- [ ] `crates/runie-testing/src/fixtures.rs:6` compiles.
- [ ] If `PermissionMode` is what the test fixture wanted, switch to that.
- [ ] If the agent `PermissionGate` is what the fixture wanted, import it from `runie_agent::permission_gate::PermissionGate` (or re-export from `runie-core`).
- [ ] `cargo check --workspace --all-targets` exits 0.
- [ ] All `runie-testing` consumers still get a working permission fixture.

## Tests

### Layer 1 — State/Logic
- N/A — import resolution.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] All tests under `crates/runie-testing/` and any consumer `tests/` directories that import these fixtures still pass.

## Files touched

- `crates/runie-testing/src/fixtures.rs`

## Notes

- Trivial `cargo fix` will resolve. Coordinate with `unify-permission-gate` (P0) which is consolidating `PermissionGate` back into `runie-core` — after that lands, the import path here becomes `runie_core::permissions::PermissionGate` again.
