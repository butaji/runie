# Delete dead `provider::Message` enum

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/provider.rs:11-50` defines `pub enum Message { System { content }, User { content }, Assistant { content, tool_calls }, ToolResult { tool_call_id, content } }` plus an `impl Message { fn role(), fn content() }` block (~39 LOC total). Its only producer is `ChatMessage::to_provider_message()` in `crates/runie-core/src/message.rs:110-128` (~19 LOC). `rg 'to_provider_message|provider::Message::' crates/` returns only the producer and tests — `Message::role()` and `Message::content()` are also never called externally.

The `Provider` trait signature at `crates/runie-core/src/provider.rs:102-105` is `fn generate(&self, messages: Vec<ChatMessage>) -> Pin<Box<dyn Stream<Item = Result<LLMEvent>> + Send + '_>>;` (and `generate_with_tools` at line 112 takes `Vec<ChatMessage>` too), so production never constructs or consumes `provider::Message`. The enum and its impl are dead weight duplicating `ChatMessage` with a different shape.

Three tests in `message.rs` (lines 259, 277, 310; ~70 LOC total) exercise `to_provider_message` exclusively — they need migration or deletion.

## Acceptance Criteria

- [ ] `pub enum Message` (provider.rs:11-26) and `impl Message { role, content }` (provider.rs:28-50) deleted.
- [ ] `pub fn to_provider_message` deleted from `crates/runie-core/src/message.rs:110-128`.
- [ ] The 3 `#[test]` cases in `message.rs` (`chat_message_to_provider_message` at line 259, `chat_message_to_provider_message_with_tool_call` at line 277, `chat_message_to_provider_message_with_tool_result_id` at line 310) deleted. They verified only the now-deleted conversion; replace with a single `chat_message_role_and_content_round_trip` test that asserts `ChatMessage { role, content }` constructs correctly.
- [ ] `rg "provider::Message\b" crates/` returns zero hits.
- [ ] `rg "\.to_provider_message\b" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `provider_message_enum_gone` — `rg "pub enum Message" crates/runie-core/src/provider.rs` returns zero hits.
- [ ] `chat_message_role_and_content_round_trip` — `ChatMessage { role: Role::User, content: "hello".into(), .. }` matches `Role::User` and `content == "hello"`.
- [ ] `provider_trait_still_uses_chat_message` — `Provider::generate` signature compiles with `Vec<ChatMessage>` argument.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_provider_trait_compiles` — `cargo check -p runie-provider` green; `Provider::generate` signature still uses `Vec<ChatMessage>`.

## Files touched

- `crates/runie-core/src/provider.rs` (~39 LOC: enum + impl deleted)
- `crates/runie-core/src/message.rs` (~19 LOC fn deleted + ~70 LOC tests deleted; ~5 LOC replacement test added)

## Notes

If a future provider needs the legacy OpenAI `Message` shape (e.g. for an external API), reintroduce the conversion as a private helper inside the relevant provider crate rather than as a shared `provider::Message` enum. Net deletion: ~120 LOC.
