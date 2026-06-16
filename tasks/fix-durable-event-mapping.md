# Fix Durable Event Mapping

**Status**: todo
**Milestone**: R3
**Category**: Sessions
**Priority**: P0

**Depends on**: flatten-event-enum
**Blocks**: (none)

## Description

`Event::to_durable()` in `runie-core/src/event/variants.rs` drops tool inputs (`Value::Null`), loses tool-result correlation (empty `id`), and never persists user messages. The durable log cannot accurately reconstruct a session.

## Acceptance Criteria

- [ ] Every durable event carries enough data to reconstruct the corresponding `ChatMessage`/`Role`/tool call.
- [ ] Tool inputs and outputs are persisted, not replaced with `Null` or empty strings.
- [ ] Round-trip tests exist for every durable variant.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_called_round_trip` — `ToolCalled` → durable → `ToolCalled` preserves input.
- [ ] `tool_result_round_trip` — `ToolResult` → durable → `ToolResult` preserves id/output/success.
- [ ] `user_message_persisted` — user messages are written to the durable log.

### Layer 2 — Event Handling
- [ ] `session_actor_persists_full_events` — `SessionActor` stores complete events.

## Files touched

- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/event/durable.rs`
- `crates/runie-core/src/session_actor.rs`
- `crates/runie-core/src/session_store.rs`

## Notes

This is a prerequisite for making `SessionStore` the single source of truth.
