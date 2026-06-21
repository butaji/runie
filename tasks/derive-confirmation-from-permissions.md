# Derive confirmation UI from permission policy

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P2

**Depends on**: unify-permission-gate
**Blocks**: none

## Description

`runie-core/src/confirmation.rs` defines `ConfirmationKind` and `ConfirmationRouter`, classifying edit/write/bash/read-only tools. `runie-core/src/permissions/mod.rs` classifies the same tools for allow/ask/deny. The two taxonomies duplicate tool-classification logic.

## Acceptance Criteria

- [ ] Confirmation UI is driven by the permission policy result and `Tool::requires_approval`.
- [ ] `ConfirmationKind` / `ConfirmationRouter` are removed or reduced to a UI label derived from the policy.
- [ ] No tool is classified differently by the two systems.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `read_only_tool_no_confirmation` — read-only tools bypass confirmation.
- [ ] `write_tool_requires_confirmation` — write tools trigger confirmation.

### Layer 2 — Event Handling
- [ ] `dialog_event_routes_by_permission_result` — confirmation dialog is shown based on permission action.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_yolo_skips_confirmation` — headless turn with auto-allow does not block on confirmation.

## Files touched

- `crates/runie-core/src/confirmation.rs`
- `crates/runie-core/src/permissions/mod.rs`
- `crates/runie-core/src/update/dialog/confirmation.rs` (if exists)
- UI confirmation code.

## Notes

Run after `unify-permission-gate` so the permission system is the single source of truth.
