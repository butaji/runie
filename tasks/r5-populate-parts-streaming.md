# Populate `Vec<Part>` during streaming

**Status**: todo
**Milestone**: R5
**Category**: Core / State
**Priority**: P1

**Depends on**: r5-add-lifecycle-events
**Blocks**: r5-per-channel-decoders, r5-parts-canonical-migration

## Description

`ChatMessage` (`crates/runie-core/src/message.rs`) has both `content: String` (monolithic) and `parts: Vec<Part>` (structured). The streaming path in `crates/runie-core/src/update/agent/core.rs` `append_response_delta` only appends to `content` — `parts` is populated only by `build_assistant_message` in `tool_parser.rs:323` when tool calls are present. This means session replay and re-rendering must reparse the monolithic string to recover part boundaries. OpenCode's `createLLMEventPublisher` (`packages/core/src/session/runner/publish-llm-event.ts:60`) accumulates text fragments and tool-call lifecycle into `SessionEvent`s that directly persist as structured parts. With the `Lifecycle` events from `r5-add-lifecycle-events`, Runie can populate `parts` during streaming: `TextStart` begins a `Part::Text`, `TextDelta` appends to it, `TextEnd` closes it; `ThinkingStart`/`ThinkingEnd` do the same for `Part::Reasoning`; `ToolCallEnd` adds a `Part::ToolCall`.

## Acceptance Criteria

- [ ] `AppState::append_response_delta` in `crates/runie-core/src/update/agent/core.rs` handles `TextStart`/`TextEnd`/`ThinkingStart`/`ThinkingEnd` events (from `r5-add-lifecycle-events`) in addition to the existing `ResponseDelta`:
  - `TextStart` pushes a new `Part::Text { content: String::new() }` to the current assistant message's `parts` (if the last part isn't already an open `Part::Text`).
  - `TextDelta` (via `ResponseDelta`) appends to both `content` (for backward compat) and the last `Part::Text` in `parts`.
  - `TextEnd` marks the current text part as closed (no-op if parts are append-only).
  - `ThinkingStart` pushes a new `Part::Reasoning { content: String::new() }`.
  - `ThinkingDelta` appends to the last `Part::Reasoning`.
  - `ThinkingEnd` closes the reasoning part.
- [ ] `finish_turn` in `core.rs` ensures `parts` is fully populated: if no `TextEnd`/`ThinkingEnd` was received, the open parts are closed (flushed). If `parts` is empty but `content` is not (legacy path without lifecycle events), a single `Part::Text { content }` is created as a fallback.
- [ ] `build_assistant_message` in `tool_parser.rs:323` still works but now appends `Part::ToolCall` to an already-populated `parts` vector (from the streaming text/reasoning parts) instead of creating `parts` from scratch.
- [ ] Session replay (`session_replay.rs`) that replays `ResponseDelta` events produces the same `parts` as the original streaming run.
- [ ] `ChatMessage.content` remains populated (backward compat for providers/consumers that still read it).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `append_response_delta_populates_text_part` — feed `[TextStart, ResponseDelta("hi"), ResponseDelta(" there"), TextEnd]` into a fresh `AppState`; the assistant message's `parts` contains `Part::Text { content: "hi there" }` and `content == "hi there"`.
- [x] `append_response_delta_populates_reasoning_part` — feed `[ThinkingStart, ThinkingDelta("reasoning"), ThinkingEnd]`; `parts` contains `Part::Reasoning { content: "reasoning" }`.
- [x] `append_response_delta_multiple_text_blocks` — feed `[TextStart, ResponseDelta("a"), TextEnd, TextStart, ResponseDelta("b"), TextEnd]`; `parts` contains two `Part::Text` entries.
- [x] `finish_turn_closes_open_parts` — feed `[TextStart, ResponseDelta("hi")]` (no `TextEnd`) then `finish_turn`; `parts` contains one `Part::Text { content: "hi" }`.
- [x] `finish_turn_fallback_creates_text_part_when_empty` — feed `[ResponseDelta("hi")]` (no lifecycle events, legacy path) then `finish_turn`; `parts` contains one `Part::Text { content: "hi" }`.
- [x] `build_assistant_message_appends_tool_call_to_existing_parts` — start with `parts = [Part::Text { content: "Let me run that" }]`, call `build_assistant_message` with one tool call; `parts` becomes `[Part::Text { ... }, Part::ToolCall { ... }]`.

### Layer 2 — Event Handling
- [ ] `stream_response_emits_lifecycle_events_to_state` — a full agent turn with a text response and no tool calls produces an assistant message with `parts = [Part::Text { content: "..." }]` and `content == "..."`.
- [ ] `stream_response_with_thinking_and_text_produces_two_parts` — a turn with thinking then text produces `parts = [Part::Reasoning { ... }, Part::Text { ... }]`.
- [ ] `session_replay_produces_same_parts` — record a turn's events, replay them into a fresh `AppState`; the replayed message's `parts` match the original.

### Layer 3 — Rendering
- [ ] `render_message_with_parts_shows_text_and_reasoning` — render a message with `parts = [Part::Reasoning { content: "thinking..." }, Part::Text { content: "answer" }]`; the `TestBackend` buffer shows both the thinking block (dimmed) and the answer text.

### Layer 4 — Smoke / Crash
- [ ] `smoke_parts_populated_during_streaming` — `cargo test -p runie-core update::agent` passes; existing agent state tests still pass.

## Files touched

- `crates/runie-core/src/update/agent/core.rs` (handle lifecycle events in `append_response_delta`/`finish_turn`, ~40 LOC change)
- `crates/runie-core/src/update/agent/mod.rs` (dispatch new event variants to `core.rs`, ~10 LOC)
- `crates/runie-core/src/tool_parser.rs` (`build_assistant_message` appends to existing `parts`, ~10 LOC change)
- `crates/runie-core/src/session_replay.rs` (ensure replay produces parts, ~5 LOC if events already replay correctly)

## Notes

Source inspiration: OpenCode `packages/core/src/session/runner/publish-llm-event.ts:60-412` (`createLLMEventPublisher` accumulates fragments into structured session events). The key insight is that `content: String` and `parts: Vec<Part>` are kept in sync during streaming — `content` is the concatenation of all `Part::Text` content (for backward compat), while `parts` is the structured representation (for replay and rendering). This task is the prerequisite for `r5-parts-canonical-migration` (which makes `parts` the sole canonical and `content` a computed getter) and `r5-per-channel-decoders` (which needs structured parts to project into channels). Keep the `content` field populated for now — the migration to `parts`-only is a separate task.
