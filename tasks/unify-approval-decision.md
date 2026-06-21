# Unify approval decision enums

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: unify-permission-gate
**Blocks**: none

## Description

`runie-core/src/proto/op.rs` defines `ApprovalDecision { Approve, Reject }` and `runie-core/src/permissions/mod.rs` defines `PermissionAction { Allow, Ask, Deny }`. Both describe allow/deny outcomes. Every protocol approval message must be mapped to a permission action, creating a redundant conversion boundary.

## Acceptance Criteria

- [ ] One canonical decision enum is used across protocol and permission systems.
- [ ] The other enum is removed or becomes a thin alias with a deprecation note.
- [ ] All protocol ops and permission evaluations use the canonical enum.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `approve_maps_to_allow` — protocol approval maps to permission allow.
- [ ] `reject_maps_to_deny` — protocol rejection maps to permission deny.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_yolo_uses_canonical_decision` — headless turn with `--yolo` still bypasses approval.

## Files touched

- `crates/runie-core/src/proto/op.rs`
- `crates/runie-core/src/permissions/mod.rs`
- `crates/runie-core/src/permissions/gate.rs`
- All callers that match on either enum.

## Notes

`PermissionAction` is the richer concept because it includes `Ask`. It is the natural canonical type.
