# Canonicalize chat-message types

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

The codebase has multiple overlapping message types: `runie_core::provider::Message`, `runie_core::message::ChatMessage`, `runie_core::proto::messages::Message`, and `runie_protocol::Message`. The first two both model System/User/Assistant/Tool roles with content and tool calls; the latter two are both JSON-RPC-ish envelopes. `ChatMessage::to_provider_message()` exists only because of the parallel representations.

## Acceptance Criteria

- [ ] `ChatMessage` becomes the single canonical conversation-message type.
- [ ] `provider::Message` is removed or reduced to a provider-specific view with `From`/`Into` only.
- [ ] The wire-protocol `Message` type has one home: either `runie-core::proto` or `runie-protocol`, not both.
- [ ] All conversions are one-way and clearly named.
- [ ] `cargo test --workspace` and `cargo check --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `chat_message_serializes_round_trip` — JSON/TOML serialization unchanged.
- [ ] `provider_view_from_chat_message` — convert canonical message to provider view and back where possible.

### Layer 2 — Event Handling
- [ ] N/A — pure type refactor.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_m3_turn_uses_canonical_messages` — replay a provider fixture through the agent and assert no conversion loss.

## Files touched

- `crates/runie-core/src/provider.rs`
- `crates/runie-core/src/message/mod.rs`
- `crates/runie-core/src/proto/messages.rs`
- `crates/runie-protocol/src/lib.rs`
- All callers that construct or match on these types.

## Notes

Coordinate with `fold-protocol-into-core` if choosing to merge `runie-protocol` into `runie-core::proto`.
