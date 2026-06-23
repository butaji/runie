# Migrate `content: String` to `parts: Vec<Part>` as canonical

**Status**: todo
**Milestone**: R5
**Category**: Core / State
**Priority**: P2

**Depends on**: r5-populate-parts-streaming
**Blocks**: none

## Description

`ChatMessage` (`crates/runie-core/src/message.rs`) currently has both `content: String` and `parts: Vec<Part>`. The streaming path (after `r5-populate-parts-streaming`) keeps them in sync, but they can drift: `build_assistant_message` creates `parts` from `content` + tool calls, the `tool_parser` fallback path sets `content` but not `parts`, and some code paths read `content` while others read `parts`. OpenHarness and OpenCode both use a content-block model where `parts` (or `content blocks`) is the sole canonical representation and the text content is a computed property. Make `parts: Vec<Part>` the canonical field; `content` becomes a computed getter that concatenates `Part::Text` content. This eliminates drift and simplifies the message model.

## Acceptance Criteria

- [ ] `ChatMessage.content` field is removed. A `pub fn content(&self) -> String` method concatenates all `Part::Text` content (and `Part::Reasoning` for reasoning content, behind a flag or separate method).
- [ ] `ChatMessage::new(role, content)` constructor creates `parts = vec![Part::Text { content }]` for backward compat.
- [ ] All code that reads `msg.content` is updated to call `msg.content()` (the getter).
- [ ] All code that writes `msg.content = ...` is updated to push a `Part::Text` to `parts` (or replace the last text part).
- [ ] `ChatMessage.tool_calls` field is removed. A `pub fn tool_calls(&self) -> Vec<&ToolCall>` method extracts from `parts` (filtering `Part::ToolCall`). The `ToolCall` struct is kept; it's just not a separate top-level field.
- [ ] `to_provider_message` in `message.rs` serializes from `parts`, not from `content`/`tool_calls`.
- [ ] Session persistence (`session_replay.rs`, `DurableCoreEvent`) serializes `parts` (already does via serde on `Part`).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `chat_message_content_getter_concatenates_text_parts` — `ChatMessage { parts: [Part::Text("a"), Part::Reasoning("r"), Part::Text("b")] }`; `content()` returns `"ab"` (reasoning excluded) or `"a\nr\nb"` (reasoning included) — pick one and document.
- [ ] `chat_message_tool_calls_getter_extracts_from_parts` — `ChatMessage { parts: [Part::Text("hi"), Part::ToolCall { id: "c1", name: "bash", args: {} }] }`; `tool_calls()` returns `[&ToolCall { id: "c1", name: "bash", ... }]`.
- [ ] `chat_message_new_creates_text_part` — `ChatMessage::new(Role::User, "hello")`; `parts` is `[Part::Text { content: "hello" }]`, `content()` is `"hello"`.
- [ ] `chat_message_no_text_parts_returns_empty_content` — `ChatMessage { parts: [Part::ToolCall { ... }] }`; `content()` is `""`.
- [ ] `to_provider_message_serializes_from_parts` — assistant message with `parts: [Part::Text("hi"), Part::ToolCall { id: "c1", name: "bash", args: {"cmd": "ls"} }]` serializes to provider message with `content: "hi"` and `tool_calls: [{id, name, args}]`.

### Layer 2 — Event Handling
- [ ] `append_response_delta_appends_to_text_part` — feed `ResponseDelta("hi")` into `AppState`; the assistant message's `parts` has a `Part::Text` with content `"hi"`, and `content()` returns `"hi"`.
- [ ] `finish_turn_with_tool_calls_has_correct_parts` — a turn with text + one tool call produces `parts: [Part::Text("Let me check"), Part::ToolCall { ... }]`; `content()` is `"Let me check"`, `tool_calls()` has one entry.

### Layer 3 — Rendering
- [ ] `render_message_from_parts_shows_all_blocks` — render a message with `parts: [Part::Reasoning("thinking"), Part::Text("answer"), Part::ToolCall { ... }]`; the `TestBackend` buffer shows thinking (dimmed), answer text, and tool-call rendering.

### Layer 4 — Smoke / Crash
- [ ] `smoke_content_field_removed` — `rg "pub content: String" crates/runie-core/src/message.rs` returns zero hits.
- [ ] `smoke_tool_calls_field_removed` — `rg "pub tool_calls:" crates/runie-core/src/message.rs` returns zero hits.
- [ ] `smoke_workspace_tests_pass` — `cargo test --workspace` passes.

## Files touched

- `crates/runie-core/src/message.rs` (remove `content`/`tool_calls` fields, add getter methods, ~60 LOC change)
- `crates/runie-core/src/message/parts.rs` (no change, already correct)
- `crates/runie-core/src/tool_parser.rs` (`build_assistant_message` pushes to `parts` only, ~10 LOC change)
- `crates/runie-core/src/update/agent/core.rs` (all `msg.content =` → `msg.push_text_part(...)`, ~20 LOC change)
- `crates/runie-core/src/provider.rs` (`to_provider_message` serializes from `parts`, ~15 LOC change)
- `crates/runie-provider/src/openai/request.rs` (`message_to_openai` reads from `parts`/getters, ~10 LOC change)
- `crates/runie-agent/src/tool_runner.rs` (`tool_result_message` pushes `Part::ToolResult`, ~5 LOC change)
- `crates/runie-agent/src/stream_response.rs` (uses `parts` for text/reasoning, ~10 LOC change)
- All other files that read `msg.content` or `msg.tool_calls` (grep and update)

## Notes

Source inspiration: OpenHarness `src/openharness/engine/messages.py:65-117` (`ConversationMessage` with `content: list[ContentBlock]` and `.text`/`.tool_uses` computed properties) and OpenCode `packages/llm/src/schema/messages.ts:275` (`Message` with `content: ContentPart[]` tagged union). This is a P2 task because it's a large codemod (every `msg.content` read/write site changes) but doesn't add new functionality — it's a cleanup that eliminates drift. Depends on `r5-populate-parts-streaming` because the streaming path must already populate `parts` before we can remove `content` as a field. Consider doing this in two commits: (1) add getters, (2) remove fields — so the first commit is reversible if tests fail. The `ToolCall` struct stays as a projection type returned by `tool_calls()`; it's just not stored as a top-level field. `Part::ToolResult` stays in `parts` alongside `Part::ToolCall`.
