# Canonicalize chat-message types

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

The codebase had multiple overlapping message types: `runie_core::provider::Message`, `runie_core::message::ChatMessage`, `runie_core::proto::messages::Message`, and `runie_protocol::Message`. The first two both modeled System/User/Assistant/Tool roles with content and tool calls.

## Acceptance Criteria

- [x] `ChatMessage` becomes the single canonical conversation-message type.
- [x] `provider::Message` is removed (does not exist).
- [x] The wire-protocol `Message` type lives in `runie-protocol` crate (IPC concern).
- [x] All conversions are one-way and clearly named.
- [x] `cargo test --workspace` and `cargo check --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `chat_message_serializes_round_trip` — JSON/TOML serialization unchanged.
- [x] `provider_view_from_chat_message` — convert canonical message to provider view and back where possible.

### Layer 2 — Event Handling
- [x] N/A — pure type refactor.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `minimax_m3_turn_uses_canonical_messages` — replay a provider fixture through the agent and assert no conversion loss.

## Files touched

- `crates/runie-core/src/message/mod.rs` — canonical ChatMessage type
- `crates/runie-protocol/src/messages.rs` — wire protocol Message type (IPC only)

## Notes

`ChatMessage` is the single canonical application-layer message type with typed roles (`Role::User`, `Role::Assistant`, etc.), content parts (`Part::Text`, `Part::ToolCall`, etc.), and provider metadata.

The wire protocol `Message` in `runie-protocol` is intentionally separate as it handles IPC serialization/deserialization for the TUI/server communication, not application semantics.

`runie-core::proto` was removed; protocol types live in `runie-protocol` crate.
