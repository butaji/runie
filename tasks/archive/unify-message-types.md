# Unify Message and Role Types

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P0

**Depends on**: unify-config-types
**Blocks**: unify-tool-result-types

## Description

Runie currently has three overlapping message types:

- `runie-core::provider::Message` — LLM API payload (`System/User/Assistant/ToolResult`).
- `runie-core::message::{Role, ChatMessage}` — UI/session message with
  metadata (`timestamp`, `id`, `provider`).
- A private `ChatMessage` in `runie-server/src/main.rs`.

The constant conversions between API and UI message types are a source of
silent data loss and role mismatches.

## Acceptance Criteria

- [x] A single canonical `ChatMessage`/`Role` type lives in `runie-core`.
- [x] `runie-core::provider` uses the canonical type (or a thin projection)
  instead of its own `Message` enum.
- [x] `runie-server` uses the canonical type from `runie-core` and deletes
  its private copy.
- [x] Serialization for sessions and for LLM APIs remains stable.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `role_as_str_matches_provider_expectations` — `Role::as_str()` returns
  the strings the provider layer needs.
- [x] `chat_message_round_trip_json` — canonical message survives
  `serde_json` round-trip with all fields.
- [x] `server_uses_core_message` — `runie-server` has no private
  `ChatMessage` struct.

### Layer 2 — Event Handling
- [x] `agent_response_builds_core_message` — `AgentEvent::Response` produces
  a canonical `ChatMessage`.

## Files touched

- `crates/runie-core/src/message.rs`
- `crates/runie-core/src/provider.rs`
- `crates/runie-core/src/llm_event.rs`
- `crates/runie-server/src/main.rs`
- All call sites that construct or pattern-match on `provider::Message`.

## Notes

`runie_core::provider::Provider::generate` now takes `Vec<ChatMessage>`.
All provider implementations (mock, openai, DynProvider) and consumers
(agent turn/headless, planner, json/server/print CLIs) were updated.
`provider::Message` remains as a conversion target via
`ChatMessage::to_provider_message()`. Added `ChatMessage` constructors for
`system`, `user`, `assistant`, `tool_result`, and `tool`.
