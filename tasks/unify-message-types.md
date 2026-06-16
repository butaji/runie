# Unify Message Types

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Two message representations exist in `runie-core`: `provider::Message` (`System`, `User`, `Assistant`, `ToolResult`) and `message::ChatMessage` + `Role`. A hand-written `to_provider_message()` conversion silently loses metadata.

## Acceptance Criteria

- [ ] A single message/role type is canonical across core.
- [ ] Provider layer uses `From`/`TryFrom` conversions instead of hand-rolled mapping.
- [ ] No metadata is silently dropped.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `message_conversion_preserves_metadata` — all fields survive `ChatMessage` ↔ `provider::Message`.
- [ ] `role_exhaustive` — every role round-trips.

## Files touched

- `crates/runie-core/src/message.rs`
- `crates/runie-core/src/provider.rs`
- `crates/runie-provider/src/*.rs`

## Notes

The old `tasks/archive/unify-message-types.md` was marked done but the duplication persists.
