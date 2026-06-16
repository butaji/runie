# Unify Permission Models

**Status**: done
**Milestone**: R3
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Approval logic is modeled twice: `runie-core/src/permissions.rs` (`PermissionSet`, `PermissionRule`) and `runie-agent/src/policy.rs` (`ToolPolicy`, `PathRule`). They evaluate the same concern with different types and no shared code.

## Acceptance Criteria

- [x] A single permission/approval model lives in `runie-core`.
- [x] `runie-agent` uses the core model; `policy.rs` is deleted or becomes a thin wrapper.
- [x] Existing behavior (allow/ask/deny, sensitive paths, read-only tools) is preserved.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `permission_set_evaluates_rules` — wildcard last-match rules work.
- [x] `policy_matches_core` — agent behavior matches core permission decisions.

## Files touched

- `crates/runie-core/src/permissions.rs`
- `crates/runie-agent/src/policy.rs`
- `crates/runie-agent/src/turn.rs`
- `crates/runie-agent/src/tools.rs`

## Notes

Coordinate with `permission-rulesets.md` if behavior needs to change.
