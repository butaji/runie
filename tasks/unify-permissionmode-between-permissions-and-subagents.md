# Unify `PermissionMode` between `permissions` and `subagents`

**Status**: done
**Milestone**: R7

**Note**: Verified 2026-06-29 — `PermissionMode` removed from `subagents/mod.rs`, re-exported from `permissions::PermissionMode`; `parse_permission_mode()` helper handles legacy camelCase names.
**Category**: Core / State
**Priority**: P2

**Depends on**: replace-remaining-custom-parsers-and-macros-with-strum
**Blocks**: unify-permission-system-rules

## Description

`permissions/mod.rs` and `subagents/mod.rs` each define a `PermissionMode` enum with identical variants and a legacy string parser. Re-export the canonical enum and move the legacy parse fallback into `FromStr` or a serde deserializer.

## Acceptance Criteria

- [x] Delete `subagents::PermissionMode`.
- [x] Re-export `permissions::PermissionMode` from `subagents`.
- [x] Legacy string parsing lives in `FromStr`.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `subagent_uses_canonical_permission_mode` — subagent config resolves to canonical enum.

## Files touched

- `crates/runie-core/src/permissions/mod.rs`
- `crates/runie-core/src/subagents/mod.rs`

## Notes

- Coordinate with `unify-permission-system-rules.md`.
## Completion Validation

All validation gates confirmed:

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (internal enum consolidation).

**Verification (2026-07-01):** `subagents/mod.rs:48` re-exports `PermissionMode` from `permissions`. Legacy `parse_permission_mode()` handles camelCase names. No duplicate `PermissionMode` enum exists.
