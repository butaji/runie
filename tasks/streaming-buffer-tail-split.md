# Streaming Buffer: Stable Region + Mutable Tail

**Status**: todo
**Milestone**: R3
**Category**: UI / Feed
**Priority**: P1

**Depends on**: event-bus-jsonl-persistence

## Description

Runie currently emits one `AgentResponse` event per LLM chunk and appends it
immediately to the conversation. Research from Codex (two-region streaming),
Aider (`MarkdownStream` stable/unstable split), and Gemini CLI
(`useGeminiStream` safe split points) shows that buffering incomplete markdown
blocks in a mutable tail prevents tearing and per-token re-renders.

## Acceptance Criteria

- [ ] `crates/runie-core/src/streaming_buffer.rs` (or `runie-tui` equivalent)
  implements a `StreamingBuffer`:
  - Accumulates incoming text deltas.
  - Tracks open markdown constructs (code fences, tables, lists).
  - Flushes **stable** completed lines/blocks to the scrollback.
  - Keeps the **unstable tail** in a mutable active cell.
  - Batches flushes on a debounced timer (e.g., 50ms) instead of per chunk.
- [ ] `AgentEvent::ResponseDelta` is transient; only the final assistant message
  is durable.
- [ ] TUI render path separates:
  - Scrollback from stable committed content.
  - Active streaming cell showing the mutable tail.
- [ ] No visible tearing when streaming code blocks, tables, or multi-line
  reasoning.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `buffer_flushes_complete_paragraph` — text ending in `\n\n` flushes.
- [ ] `buffer_holds_incomplete_code_fence` — text inside ``` stays in tail.
- [ ] `buffer_batches_deltas` — 10 deltas within 50ms produce one flush.

### Layer 2 — Event Handling
- [ ] `response_delta_updates_tail` — transient event updates active cell.

### Layer 3 — Rendering
- [ ] `stable_content_in_scrollback` — committed text appears above input.
- [ ] `tail_content_in_active_cell` — incomplete text appears in active cell.

## Notes

**Why not a crate:**
- `crate-replacement-audit` evaluated `ratatui-markdown`. Markdown streaming
  with tool-call interleaving is Runie-specific; a custom buffer is justified.

**Files touched:**
- `crates/runie-core/src/streaming_buffer.rs` (new)
- `crates/runie-tui/src/message/mod.rs`
- `crates/runie-tui/src/ui.rs`

**Out of scope:**
- Real-time word-wrap caching (future optimization).
- Sixel image streaming.
