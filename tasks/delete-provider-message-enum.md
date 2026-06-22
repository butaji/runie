# Delete dead `provider::Message` enum

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/provider.rs:11-26` defines `pub enum Message { System { content }, User { content }, Assistant { content, tool_calls }, ToolResult { tool_call_id, content } }` (~26 LOC). Its only producer is `ChatMessage::to_provider_message()` in `crates/runie-core/src/message.rs:108-128`, which is `#[cfg(test)]`-only in spirit and has zero production callers — `rg 'to_provider_message|provider::Message::' crates/` returns only the producer and tests.

The `Provider` trait signature is `fn generate(&self, messages: Vec<ChatMessage>)`, so production never constructs or consumes `provider::Message`. The enum is dead weight duplicating `ChatMessage` with a different shape.

## Acceptance Criteria

- [ ] `pub enum Message` deleted from `crates/runie-core/src/provider.rs`.
- [ ] `pub fn to_provider_message` deleted from `crates/runie-core/src/message.rs`.
- [ ] The 3 `#[test]` cases in `message.rs` (`chat_message_to_provider_message`, `chat_message_to_provider_message_with_tool_call`, `chat_message_to_provider_message_with_tool_result_id`) migrated to assert against `ChatMessage` shape directly, or deleted if they only verified the now-deleted conversion.
- [ ] `rg "provider::Message\b" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `provider_message_enum_gone` — `rg "pub enum Message" crates/runie-core/src/provider.rs` returns zero hits.
- [ ] `chat_message_constructs_match_expected_role` — asserts that `ChatMessage { role: Role::User, content: "hello".into() }` matches `Role::User` (replacing the deleted `to_provider_message` test).

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_provider_trait_uses_chat_message` — `cargo check -p runie-provider` green; `Provider::generate` signature still uses `Vec<ChatMessage>`.

## Files touched

- `crates/runie-core/src/provider.rs`
- `crates/runie-core/src/message.rs`

## Notes

If a future provider needs the legacy OpenAI `Message` shape (e.g. for an external API), reintroduce the conversion as a private helper inside the relevant provider crate rather than as a shared `provider::Message` enum.
