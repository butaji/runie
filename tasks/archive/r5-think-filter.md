# Add `ThinkFilter` for inline `<tool_call>`/`</thinking>` tag handling

**Status**: done
**Milestone**: R5
**Category**: Core / State
**Priority**: P1

**Depends on**: r5-add-lifecycle-events
**Blocks**: none

## Description

Many providers (DeepSeek, vLLM, OpenRouter, local Ollama models) stream reasoning content as plain text wrapped in `<tool_call>`/`</thinking>` or `<thinking>`/`</thinking>` tags inside the same `TextDelta` stream, with no separate `ThinkingDelta` channel. Runie's `stream_response.rs` only handles `ThinkingDelta` events — inline tags end up in the assistant text content and get displayed to the user. Goose solves this with a `ThinkFilter` state machine (`crates/goose/src/providers/base.rs:40-164`) that buffers streaming chunks, detects opening/closing tags (including tags split across chunk boundaries), and routes content into `content` vs `thinking` buckets. Add the same filter as a stream transformer that sits between the provider's raw `LLMEvent` stream and `stream_response`, converting inline `TextDelta` tags into proper `ThinkingStart`/`ThinkingDelta`/`ThinkingEnd` events.

## Acceptance Criteria

- [ ] New module `crates/runie-agent/src/think_filter.rs` declares `pub struct ThinkFilter` with `pub fn new() -> Self` and `pub fn feed(&mut self, event: LLMEvent) -> Vec<LLMEvent>`.
- [ ] `feed` on `TextDelta(delta)` scans for `<tool_call>`, `</thinking>`, `<thinking>`, `</thinking>` tags. Text before an opening tag is emitted as `TextDelta`. Text inside tags is emitted as `ThinkingDelta` (wrapped with `ThinkingStart`/`ThinkingEnd` via the `Lifecycle` state machine from `r5-add-lifecycle-events`). Text after a closing tag is emitted as `TextDelta`.
- [ ] Partial tags split across deltas are buffered: if a delta ends with `<tool_call>` (a prefix of `<tool_call>`), the partial bytes are held back until the next delta resolves whether it's a real tag or plain text.
- [ ] `feed` on `ThinkingDelta(delta)` passes through unchanged (structured reasoning from providers that support it is not double-processed).
- [ ] `feed` on `ToolCallStart`/`ToolCallInputDelta`/`ToolCallEnd`/`Usage`/`Finish`/`Error` flushes any buffered partial tag as `TextDelta`, then passes the event through unchanged.
- [ ] `pub fn flush(&mut self) -> Vec<LLMEvent>` drains any remaining buffer (partial tags, open thinking blocks) at stream end.
- [ ] `stream_response()` in `crates/runie-agent/src/stream_response.rs` wraps the provider stream: `provider.generate(...).map(|ev| think_filter.feed(ev)).flatten()` before feeding into `StreamState::handle_event`. The `ThinkFilter` is constructed per turn and `flush()`ed after the stream ends.
- [ ] Existing replay fixtures that don't contain inline thinking tags produce identical `LLMEvent` sequences (no regression).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `think_filter_passes_plain_text_unchanged` — `feed(TextDelta("hello world"))` returns `[TextDelta("hello world")]`.
- [ ] `think_filter_extracts_closed_thinking_block` — `feed(TextDelta("before <tool_call>\nreasoning\n</thinking>\nafter"))` returns `[TextDelta("before "), ThinkingStart, ThinkingDelta("\nreasoning\n"), ThinkingEnd, TextDelta("after")]`.
- [ ] `think_filter_handles_angle_bracket_tags` — same as above with `<thinking>`/`</thinking>` instead of <tool_call>/`</thinking>`.
- [ ] `think_filter_buffers_partial_open_tag` — `feed(TextDelta("hi <tool_call>"))` returns `[TextDelta("hi ")]` (partial `<tool_call>` held back); `feed(TextDelta(">\nreasoning"))` returns `[ThinkingStart, ThinkingDelta("\nreasoning")]`.
- [ ] `think_filter_buffers_partial_close_tag` — inside a thinking block, `feed(TextDelta("reasoning </think"))` returns `[ThinkingDelta("reasoning ")]`; `feed(TextDelta("ing>\nafter"))` returns `[ThinkingEnd, TextDelta("after")]`.
- [ ] `think_filter_passes_structured_thinking_delta_unchanged` — `feed(ThinkingDelta("hi"))` returns `[ThinkingDelta("hi")]`.
- [ ] `think_filter_flush_drains_open_block` — after `feed(TextDelta("<tool_call>\nunfinished"))`, `flush()` returns `[ThinkingStart, ThinkingDelta("\nunfinished"), ThinkingEnd]`.
- [ ] `think_filter_flush_drains_partial_tag_as_text` — after `feed(TextDelta("hi <tool_call>"))` with no resolution, `flush()` returns `[TextDelta("<tool_call>")]` (treat unresolved partial as literal text).
- [ ] `think_filter_nested_tags_track_depth` — `feed(TextDelta("<tool_call> inner <tool_call> deep </thinking> </thinking>"))` routes correctly (Goose tracks nesting depth; we can choose to flatten to one block for simplicity, but must not lose content).

### Layer 2 — Event Handling
- [ ] `stream_response_with_think_filter_extracts_inline_reasoning` — feed a provider stream of `[TextDelta("Here is <tool_call>\nmy reasoning\n</thinking>\nThe answer is 42")]` through `stream_response`; `StreamedResponse.text` equals `"Here is The answer is 42"` and `StreamedResponse.reasoning` equals `"\nmy reasoning\n"`.
- [ ] `stream_response_with_think_filter_no_regression_without_tags` — existing replay fixtures (minimax, openai) produce identical `StreamedResponse` as before.

### Layer 3 — Rendering
- N/A — rendering impact is via the `StreamedResponse.reasoning` field which the UI already displays.

### Layer 4 — Smoke / Crash
- [ ] `smoke_think_filter_module_present` — `ls crates/runie-agent/src/think_filter.rs` succeeds; workspace builds.

## Files touched

- `crates/runie-agent/src/think_filter.rs` (new, ~140 LOC)
- `crates/runie-agent/src/lib.rs` (add `mod think_filter;`)
- `crates/runie-agent/src/stream_response.rs` (wrap provider stream through `ThinkFilter`, ~10 LOC change)

## Notes

Source inspiration: Goose `crates/goose/src/providers/base.rs:40-164` (`ThinkFilter`, 125 LOC) and gptme `gptme/llm/__init__.py:594-751` (char-level state machine). The Rust version uses `&str` scanning instead of char-by-char iteration for performance — tags are ASCII, so `str::find` is sufficient. Use `LifecycleState` from `r5-add-lifecycle-events` to emit the `ThinkingStart`/`ThinkingEnd` wrappers so downstream consumers don't need to know whether reasoning came from a structured provider channel or an inline tag. The filter is agent-side (not provider-side) because it's a presentation-layer normalization: a provider that emits structured `ThinkingDelta` should not be filtered, while a provider that emits inline tags should. The `stream_response` function is the single chokepoint where both paths merge. If `ThinkFilter` is agent-side, providers stay thin.
