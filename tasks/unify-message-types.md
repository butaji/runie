# Unify Message and Role Types

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P0

**Depends on**: (none)
**Blocks**: `unify-tool-result-types`

## Description

Runie currently has three overlapping message types:

- `runie-core::provider::Message` — LLM API payload (`System/User/Assistant/ToolResult`).
- `runie-core::message::{Role, ChatMessage}` — UI/session message with
  metadata (`timestamp`, `id`, `provider`).
- A private `ChatMessage` in `runie-server/src/main.rs`.

The constant conversions between API and UI message types are a source of
silent data loss and role mismatches.

## Acceptance Criteria

- [ ] A single canonical `ChatMessage`/`Role` type lives in `runie-core`.
- [ ] `runie-core::provider` uses the canonical type (or a thin projection)
  instead of its own `Message` enum.
- [ ] `runie-server` uses the canonical type from `runie-core` and deletes
  its private copy.
- [ ] Serialization for sessions and for LLM APIs remains stable.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `role_as_str_matches_provider_expectations` — `Role::as_str()` returns
  the strings the provider layer needs.
- [ ] `chat_message_round_trip_json` — canonical message survives
  `serde_json` round-trip with all fields.
- [ ] `server_uses_core_message` — `runie-server` has no private
  `ChatMessage` struct.

### Layer 2 — Event Handling
- [ ] `agent_response_builds_core_message` — `AgentEvent::Response` produces
  a canonical `ChatMessage`.

## Files touched

- `crates/runie-core/src/message.rs`
- `crates/runie-core/src/provider.rs`
- `crates/runie-core/src/llm_event.rs`
- `crates/runie-server/src/main.rs`
- All call sites that construct or pattern-match on `provider::Message`.

## Notes

The provider layer may need a helper like `fn to_api_messages(&[ChatMessage])`
to produce the exact shape each provider expects, but the source of truth
should be the canonical `ChatMessage`.
