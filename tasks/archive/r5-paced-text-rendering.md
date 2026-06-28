# Add paced text rendering for smooth typing animation

**Status**: done
**Milestone**: R5
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Text currently appears in the TUI as it arrives from the 50ms debounced `StreamingBuffer::flush()`. When the provider sends a burst of deltas (e.g., 500 characters in one chunk), the entire block appears instantly, creating a jarring dump. When the provider is slow, the text trickles in line-by-line. OpenCode's `createPacedValue` (`packages/ui/src/components/message-part.tsx:209-262`) renders text in chunks of 2-24 characters at 24ms intervals, snapping to word boundaries, creating a smooth typing animation regardless of provider speed. Add a `PacedRenderer` in the TUI layer that decouples "received text" from "displayed text", advancing the display cursor on each render tick.

## Acceptance Criteria

- [ ] New struct `PacedRenderer` in `crates/runie-tui/src/pace.rs` with:
  - `pub fn new() -> Self` — initializes with empty received and displayed buffers.
  - `pub fn push(&mut self, text: &str)` — appends to the received buffer.
  - `pub fn tick(&mut self) -> String` — advances the displayed cursor by `max(2, rate_chunk)` characters, snapping to the next word boundary (whitespace or punctuation). Returns the full displayed text so far. If displayed == received, returns the full text without change.
  - `pub fn finish(&mut self)` — flushes all remaining received text to displayed (for when the stream ends).
  - `pub fn displayed(&self) -> &str` — returns the current displayed text.
  - `pub fn pending(&self) -> &str` — returns the received text not yet displayed.
- [ ] The chunk size is adaptive: `rate_chunk = clamp(received.len() / 20, 2, 24)`. This means a 500-char burst renders in ~20 ticks (~480ms at 24ms intervals), while a 20-char delta renders in ~10 ticks (~240ms).
- [ ] Word-boundary snapping: if the next character after the chunk is not a whitespace/punctuation, advance to the next whitespace/punctuation (within a 10-char lookahead). If no boundary is found, emit the raw chunk (don't block on long words).
- [ ] `PacedRenderer` is integrated into the message feed rendering: the current assistant message's text is routed through a `PacedRenderer` during streaming. On each UI render tick (the existing render loop), `tick()` is called and the returned text is what gets rendered.
- [ ] When the stream ends (`TurnComplete`), `finish()` is called so the full text is displayed immediately.
- [ ] Non-streaming messages (replayed from history, loaded sessions) bypass the `PacedRenderer` and display instantly.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `paced_renderer_starts_empty` — `PacedRenderer::new()`; `displayed()` is `""`, `pending()` is `""`.
- [ ] `paced_renderer_tick_advances_cursor` — `push("hello world")`, `tick()` returns `"hel"` (3 chars, snapped to word boundary at `"lo"` — or whatever the chunk size dictates); `pending()` is `"lo world"`.
- [ ] `paced_renderer_tick_snaps_to_word_boundary` — `push("hello wonderful world")`, first `tick()` advances to `"hello "` (snaps at the space after "hello").
- [ ] `paced_renderer_tick_catches_up_on_small_input` — `push("hi")`, `tick()` returns `"hi"` (chunk size >= input size).
- [ ] `paced_renderer_finish_flushes_all` — `push("hello world")`, `finish()`, `displayed()` is `"hello world"`, `pending()` is `""`.
- [ ] `paced_renderer_adaptive_chunk_size` — `push(&"a".repeat(500))`, first `tick()` advances by ~25 chars (500/20 = 25, clamped to 24); `push("hi")`, `tick()` advances by 2 chars (min chunk size).
- [ ] `paced_renderer_tick_no_op_when_caught_up` — `push("hi")`, `tick()` returns `"hi"`, second `tick()` returns `"hi"` (no change).

### Layer 2 — Event Handling
- [ ] `paced_renderer_integrated_with_response_delta` — feed `[ResponseDelta("hello "), ResponseDelta("world")]` into the UI; on the first render tick, `displayed()` is a prefix of `"hello world"`; after `finish()`, `displayed()` is `"hello world"`.
- [ ] `paced_renderer_finish_on_turn_complete` — feed `TurnComplete` event; `PacedRenderer::finish()` is called, full text displayed.

### Layer 3 — Rendering
- [ ] `render_paced_text_shows_partial_during_streaming` — using `TestBackend`, simulate a streaming message with `PacedRenderer` at tick 3 of 10; the buffer shows a partial text string (not the full received text).
- [ ] `render_paced_text_shows_full_after_finish` — after `finish()`, the buffer shows the full text.

### Layer 4 — Smoke / Crash
- [ ] `smoke_pace_module_present` — `ls crates/runie-tui/src/pace.rs` succeeds; workspace builds.

## Files touched

- `crates/runie-tui/src/pace.rs` (new, ~80 LOC)
- `crates/runie-tui/src/lib.rs` (add `mod pace;`)
- `crates/runie-tui/src/ui/messages/mod.rs` (route streaming message text through `PacedRenderer`, ~20 LOC)
- `crates/runie-tui/src/ui_actor.rs` (call `tick()` on each render cycle, `finish()` on `TurnComplete`, ~10 LOC)

## Notes

Source inspiration: OpenCode `packages/ui/src/components/message-part.tsx:209-262` (`createPacedValue`, 2-24 chars at 24ms intervals with `TEXT_RENDER_SNAP` boundary detection). The Rust version uses the existing render tick (the `UiActor`'s render loop) instead of a separate timer — one `tick()` call per render frame. The 24ms interval in OpenCode is the timer interval; in Runie, the render loop already runs at ~60fps (16ms) or ~30fps (33ms), so one `tick()` per frame is equivalent. This is a P2 polish task — it doesn't affect correctness, only perceived smoothness. It's independent of other R5 tasks and can be done in parallel. Keep the `PacedRenderer` scoped to the current streaming message only — completed messages render instantly from `AppState`.
