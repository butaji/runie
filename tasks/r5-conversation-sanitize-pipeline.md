# Add post-turn conversation sanitization pipeline

**Status**: todo
**Milestone**: R5
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

After a streaming turn completes, the message history can contain edge cases that break subsequent provider calls: dangling `Part::ToolCall` blocks without matching `Part::ToolResult` (if tool execution was interrupted), orphaned `Part::ToolResult` blocks whose tool-call id doesn't match any assistant message, empty assistant messages from failed streams, and consecutive same-role messages that some providers reject. Runie's `crates/runie-provider/src/openai/normalize.rs` merges consecutive same-role messages and ensures history starts with User/System, but it doesn't validate tool-call/tool-result pairing. Goose applies a pipeline of 8 fixers after each turn (`crates/goose-providers/src/conversation/mod.rs:199-257`). Add a `sanitize_messages` pipeline that runs after each turn in the agent loop, before the next provider call.

## Acceptance Criteria

- [ ] New module `crates/runie-core/src/sanitize.rs` declares `pub fn sanitize_messages(messages: &mut Vec<ChatMessage>)` that applies, in order:
  1. `remove_empty_assistant_messages` — drops `ChatMessage { role: Assistant, content: "", parts: [], tool_calls: [] }`.
  2. `remove_dangling_tool_calls` — for each assistant message, removes `Part::ToolCall` blocks whose id has no matching `Part::ToolResult` in a subsequent message. Logs a warning for each removed block.
  3. `remove_orphan_tool_results` — for each tool-result message, removes `Part::ToolResult` blocks whose id doesn't match any `Part::ToolCall` in a preceding assistant message.
  4. `merge_consecutive_same_role` — merges consecutive `User`/`System` messages by concatenating content and extending `parts`. (Already done in `normalize.rs` for the OpenAI provider; move it here so all providers benefit.)
  5. `ensure_starts_with_user_or_system` — if the first message is `Assistant` or `Tool`, prepends a `ChatMessage::system("Continue.")` placeholder.
  6. `trim_assistant_whitespace` — trims leading/trailing whitespace from assistant `content` and `Part::Text` content.
- [ ] `run_agent_turn_with_skills` in `crates/runie-agent/src/turn.rs` calls `sanitize_messages(&mut messages)` after each iteration's tool execution, before the next `stream_response` call.
- [ ] `crates/runie-provider/src/openai/normalize.rs` `normalize_messages` delegates to `sanitize_messages` for the shared fixers (remove empty, merge consecutive, ensure first) and keeps only OpenAI-specific normalization (stripping `provider_metadata`).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `sanitize_removes_empty_assistant` — input `[Assistant("")]`, output `[]`.
- [ ] `sanitize_removes_dangling_tool_call` — input `[Assistant with ToolCall(id="c1"), User("hi")]`, output `[Assistant with no ToolCall, User("hi")]` (the dangling call is removed).
- [ ] `sanitize_keeps_matched_tool_call_and_result` — input `[Assistant with ToolCall(id="c1"), ToolResult(id="c1")]`, output unchanged.
- [ ] `sanitize_removes_orphan_tool_result` — input `[User("hi"), ToolResult(id="orphan")]`, output `[User("hi")]` (orphan result removed).
- [ ] `sanitize_merges_consecutive_user` — input `[User("a"), User("b")]`, output `[User("a\nb")]` (or `User("a\n\nb")` — pick one and document).
- [ ] `sanitize_prepends_placeholder_when_first_is_assistant` — input `[Assistant("hi")]`, output `[System("Continue."), Assistant("hi")]`.
- [ ] `sanitize_trims_assistant_whitespace` — input `[Assistant("  hi  ")]`, output `[Assistant("hi")]`.
- [ ] `sanitize_pipeline_is_idempotent` — running `sanitize_messages` twice produces the same output as running it once.

### Layer 2 — Event Handling
- [ ] `agent_turn_calls_sanitize_after_tool_execution` — in a test turn with one tool call, after `execute_tools` returns, `sanitize_messages` has been called (verify via a spy or by checking that a deliberately dangling tool call is removed before the next iteration).

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_sanitize_module_present` — `ls crates/runie-core/src/sanitize.rs` succeeds; workspace builds.

## Files touched

- `crates/runie-core/src/sanitize.rs` (new, ~120 LOC)
- `crates/runie-core/src/lib.rs` (add `pub mod sanitize;`)
- `crates/runie-agent/src/turn.rs` (call `sanitize_messages` after tool execution, ~3 LOC)
- `crates/runie-provider/src/openai/normalize.rs` (delegate shared fixers to `sanitize`, ~20 LOC reduction)

## Notes

Source inspiration: Goose `crates/goose-providers/src/conversation/mod.rs:199-257` (`fix_conversation` pipeline, 8 fixers). Runie doesn't need Goose's `fix_empty_tool_results` (we construct tool results explicitly in `tool_runner.rs`) or `fix_lead_trail` (covered by `trim_assistant_whitespace`). The pipeline runs in the agent loop, not the provider layer, so it benefits all providers without each one reimplementing it. Keep `sanitize_messages` taking `&mut Vec<ChatMessage>` (in-place) to avoid cloning the entire history on every turn.
