# Streaming Buffer: Stable Region + Mutable Tail

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: event-bus-jsonl-persistence

## Description

`crates/runie-core/src/streaming_buffer.rs` implements a `StreamingBuffer` that accumulates text deltas, tracks open markdown constructs, and flushes stable content. It is now wired into the TUI render path so streaming assistant responses show stable committed content in the scrollback and the mutable tail appended to the last message.

## Acceptance Criteria

- [x] `AgentEvent::ResponseDelta` is emitted as a transient event and does not become a durable `MessageSent` until the response finishes.
  - Added `ResponseDelta` variant to `AgentEvent` for streaming deltas (transient, not persisted)
  - Kept `Response` variant for complete responses (persisted as `MessageSent`)
  - Updated `to_durable()` to return `None` for `ResponseDelta`
  - `stream_response()` emits `ResponseDelta` for each delta, `Response` on completion
- [x] `AppState` holds a `StreamingBuffer` for the active assistant response.
  - Added `streaming_buffer: StreamingBuffer` field to `AgentState`
  - `append_response_delta()` pushes to buffer and flushes stable content
  - `set_thinking()` resets the buffer for new turns
  - `finish_turn()` forces flush of remaining tail and resets buffer
- [x] `Snapshot` includes `streaming_tail` for TUI rendering.
  - Added `streaming_tail: String` to `Snapshot`
  - `fill_snapshot_agent()` copies tail from buffer
- [x] `StreamingBuffer` has `has_pending_content()` method.
- [x] TUI render path renders the tail appended to messages when `turn_active` is true.
  - Modified `build_lines_with_mapping()` to append streaming tail after committed content
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `buffer_flushes_complete_paragraph` — text ending in `\n\n` flushes.
- [x] `buffer_holds_incomplete_code_fence` — text inside ``` stays in tail.
- [x] `buffer_batches_deltas` — multiple deltas within the debounce window produce one flush.

### Layer 2 — Event Handling
- [x] `response_delta_updates_tail` — transient event updates the active cell (via StreamingBuffer).

### Layer 3 — Rendering
- [x] `streaming_tail_renders_when_turn_active` — tail content appears in rendered output.
- [x] `stable_content_in_scrollback` — committed text appears above input.

### Layer 4 — Smoke / Crash
- [x] `cargo build --release` succeeds.

## Files touched

- `crates/runie-core/src/streaming_buffer.rs`
- `crates/runie-core/src/state.rs`
- `crates/runie-core/src/event/agent.rs`
- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/snapshot.rs`
- `crates/runie-core/src/model/cache.rs`
- `crates/runie-core/src/update/agent.rs`
- `crates/runie-tui/src/ui/messages.rs`
- `crates/runie-tui/src/tests/render/transient.rs`

## Notes

- The buffer implementation and TUI integration are complete.
- Future enhancement: render streaming tail in a distinct visual style (e.g., dimmed or with animation).
