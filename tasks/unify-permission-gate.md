# Unify duplicated PermissionGate

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P0

**Depends on**: none
**Blocks**: unify-approval-decision, derive-confirmation-from-permissions

## Description

`PermissionGate` is implemented twice: once in `runie-core/src/permissions/gate.rs` and again in `runie-agent/src/permission_gate.rs`. The two files are byte-for-byte identical except for import paths. Both are exported publicly, and `runie-testing` imports the core copy while `runie-agent` re-exports its own. Any permission fix must be applied in two places, and callers can silently pick the wrong crate.

## Acceptance Criteria

- [ ] `crates/runie-agent/src/permission_gate.rs` is deleted.
- [ ] `runie-agent` re-exports `PermissionGate` from `runie_core::permissions::gate::PermissionGate`.
- [ ] `runie-testing` and all other callers use the core type consistently.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `permission_gate_evaluate_allows_read_only` — verifies the gate still returns the expected `PermissionResult` for read-only tools.
- [ ] `permission_gate_evaluate_asks_for_write` — verifies write/edit/bash tools still require approval.

### Layer 2 — Event Handling
- [ ] N/A — no event handling change.

### Layer 3 — Rendering
- [ ] N/A — no rendering change.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_turn_uses_core_permission_gate` — run a headless turn with a mock provider and confirm permission decisions come from the core gate.

## Files touched

- `crates/runie-agent/src/permission_gate.rs`
- `crates/runie-agent/src/lib.rs`
- `crates/runie-testing/src/fixtures.rs`
- `crates/runie-testing/src/runner.rs`

## Notes

Zero-risk consolidation. The core crate is the natural home for permission logic because `runie-agent` already depends on `runie-core`.
