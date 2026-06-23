# Add lifecycle open/close events to `LLMEvent`

**Status**: done
**Milestone**: R5
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: r5-think-filter, r5-populate-parts-streaming

## Description

`LLMEvent` (`crates/runie-core/src/llm_event.rs:15-35`) currently has `TextDelta(String)` and `ThinkingDelta(String)` with no open/close pairing. Providers that emit content-block boundaries (Anthropic `content_block_start`/`content_block_stop`, OpenAI Responses `output_item.added`/`output_item.done`) have no way to signal when a text or reasoning block begins and ends. The state-update layer therefore cannot know when to start a new `Part::Text` vs append to the previous one — it can only concatenate into a monolithic `content: String`. OpenCode solves this with a `Lifecycle` state machine (`packages/llm/src/protocols/utils/lifecycle.ts`) that tracks open block ids and emits synthetic `text-start`/`text-end` events when a provider doesn't signal boundaries. Add the same event variants and a `Lifecycle` helper so downstream consumers (the agent stream aggregator, the UI actor) can build proper `Vec<Part>` content during streaming.

## Acceptance Criteria

- [x] `LLMEvent` gains four variants: `TextStart`, `TextEnd`, `ThinkingStart`, `ThinkingEnd`. `TextDelta` and `ThinkingDelta` remain.
- [x] New module `crates/runie-core/src/lifecycle.rs` declares `LifecycleState` with `new()`, `text_delta(&mut self, id: &str, delta: &str) -> Vec<LLMEvent>`, `thinking_delta(&mut self, id: &str, delta: &str) -> Vec<LLMEvent>`, `text_end(&mut self, id: &str) -> Vec<LLMEvent>`, `thinking_end(&mut self, id: &str) -> Vec<LLMEvent>`, and `finish(&mut self, reason: StopReason) -> Vec<LLMEvent>` (closes all open blocks).
- [x] `text_delta` emits `[TextStart { id }, TextDelta { id, delta }]` when `id` is not already open, otherwise `[TextDelta { id, delta }]`.
- [x] `finish` emits `TextEnd`/`ThinkingEnd` for every open block, then a single `Finish` event.
- [x] `crates/runie-provider/src/openai/stream.rs` wraps its `TextDelta`/`ThinkingDelta` emissions through a `LifecycleState` so structured `TextStart`/`TextEnd` events appear even though OpenAI Chat doesn't signal block boundaries.
- [x] `crates/runie-agent/src/stream_response.rs` `StreamState::handle_event` handles the four new variants: `TextStart`/`ThinkingStart` are no-ops (the accumulator already keys by id); `TextEnd`/`ThinkingEnd` flush the current text/reasoning accumulator. Existing `TextDelta`/`ThinkingDelta` behaviour unchanged.
- [x] Existing `LLMEvent` serde serialization stays backward-compatible: new variants serialize with `type: "textStart"` etc. and deserialize without breaking old fixtures.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds (including existing replay fixtures).

## Tests

### Layer 1 — State/Logic
- [x] `lifecycle_emits_start_on_first_delta` — `text_delta("b1", "hi")` returns `[TextStart { id: "b1" }, TextDelta { id: "b1", delta: "hi" }]`.
- [x] `lifecycle_skips_start_on_continuation` — after `text_delta("b1", "hi")`, a second `text_delta("b1", " world")` returns `[TextDelta { id: "b1", delta: " world" }]` (no duplicate `TextStart`).
- [x] `lifecycle_finish_closes_all_open_blocks` — two text blocks and one thinking block open; `finish()` returns three `*End` events.
- [x] `lifecycle_text_end_removes_from_open_set` — after `text_end("b1")`, a subsequent `text_delta("b1", "x")` emits `TextStart` again.
- [x] `lifecycle_thinking_delta_emits_thinking_start` — symmetric to text.

### Layer 2 — Event Handling
- [x] `stream_response_handles_text_start_and_end` — feed `[TextStart { id: "b1" }, TextDelta { id: "b1", delta: "hi" }, TextEnd { id: "b1" }]` into `StreamState::handle_event`; the resulting `StreamedResponse.text` equals `"hi"`.
- [x] `stream_response_handles_thinking_start_and_end` — same for thinking; `StreamedResponse.reasoning` equals the concatenated deltas.
- [x] `openai_stream_emits_text_start_before_first_delta` — existing OpenAI stream test fixture produces `TextStart` as the first text event.

### Layer 3 — Rendering
- N/A — rendering impact is via `r5-populate-parts-streaming`.

### Layer 4 — Smoke / Crash
- [x] `smoke_lifecycle_module_present` — `ls crates/runie-core/src/lifecycle.rs` succeeds; workspace builds.
- [x] `smoke_existing_replay_fixtures_still_pass` — `cargo test -p runie-agent` replay tests (minimax, openai fixtures) pass unchanged.

## Files touched

- `crates/runie-core/src/llm_event.rs` (add four variants, ~10 LOC)
- `crates/runie-core/src/lifecycle.rs` (new, ~90 LOC)
- `crates/runie-core/src/lib.rs` (add `pub mod lifecycle;`)
- `crates/runie-provider/src/openai/stream.rs` (wrap emissions through `LifecycleState`, ~15 LOC change)
- `crates/runie-agent/src/stream_response.rs` (handle new variants in `handle_event`, ~10 LOC)
- `crates/runie-core/src/session_replay.rs` (no changes needed - new variants handled as no-ops in agent layer)

## Notes

Source inspiration: OpenCode `packages/llm/src/protocols/utils/lifecycle.ts` (102 LOC). The Rust version drops the `stepStarted`/`stepFinish` coupling — Runie already has `Thinking`/`ThoughtDone`/`TurnComplete` events at the agent layer for that. Keep `LifecycleState` pure (no I/O, no event-bus emission) so it can be unit-tested without an actor. The `id` parameter is `&str` — for OpenAI Chat which has no block ids, use a synthetic `"text"` or `"reasoning"` id; for Anthropic, use the `content_block` index. This task is the prerequisite for `r5-populate-parts-streaming` (which needs `TextEnd` to flush a `Part::Text`) and `r5-think-filter` (which needs `ThinkingStart`/`ThinkingEnd` to delimit extracted thinking blocks).
