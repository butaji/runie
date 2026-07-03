# Unify approval decision enums

**Status**: done
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: unify-permission-gate
**Blocks**: none

## Description

`runie-protocol/src/op.rs` defines `ApprovalDecision { Allow, Deny }` and `runie-core/src/permissions/mod.rs` defines `PermissionAction { Allow, Ask, Deny }`. Both describe allow/deny outcomes. Since `runie-protocol` cannot depend on `runie-core` (to avoid circular dependencies), `PermissionAction` is established as the canonical type in core with a `From<ApprovalDecision>` conversion.

## Acceptance Criteria

- [x] `PermissionAction` is the canonical decision enum used throughout core and agent.
- [x] `From<ApprovalDecision>` conversion is provided in `runie-core/src/permissions/mod.rs`.
- [x] Documentation clarifies that `ApprovalDecision` is protocol-only and is converted at the boundary.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `approval_decision_allow_maps_to_permission_allow` — protocol approval maps to permission allow.
- [x] `approval_decision_deny_maps_to_permission_deny` — protocol rejection maps to permission deny.
- [x] `permission_action_canonical` — verifies all variants exist and work correctly.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A — headless yolo mode already uses `build_sink(yolo)` which bypasses approval; no change needed.

## Files touched

- `crates/runie-core/src/permissions/mod.rs` — added `From<ApprovalDecision>` conversion and documentation.
- `crates/runie-core/src/permissions/tests.rs` — added conversion tests.

## Notes

`PermissionAction` is the richer concept because it includes `Ask`. It is the natural canonical type used throughout core and agent. The `ApprovalDecision` type remains in protocol for serialization compatibility, but is converted to `PermissionAction` at the protocol/core boundary.
