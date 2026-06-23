# Add live markdown healing to `StreamingBuffer`

**Status**: todo
**Milestone**: R5
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`StreamingBuffer` (`crates/runie-core/src/streaming_buffer.rs:1-196`) splits streaming content into stable lines and a tail, and detects open code fences and tables. But it does not heal incomplete markdown syntax: an unclosed `**bold**` span, an unclosed `` `code` `` span, or an unclosed `[link](` all render as raw text until the closing bytes arrive. OpenCode's `markdown-stream.ts` uses the `remend` library to close incomplete syntax for display (`hello **world` → `hello **world**`) while keeping the raw text for future deltas. Aider's `MarkdownStream` (`aider/mdstream.py:92-243`) uses a sliding window of `live_window=6` lines in a `rich.live.Live` component, printing stable lines above and rendering only the unstable tail. Add a `heal_markdown` step to `StreamingBuffer::flush` that closes incomplete inline syntax in the stable lines before they're emitted to the renderer, while keeping the raw tail untouched.

## Acceptance Criteria

- [ ] New function `pub fn heal_markdown(text: &str) -> String` in `crates/runie-core/src/streaming_buffer.rs` (or a new `markdown/heal.rs` submodule). It closes, for display purposes only:
  - Unclosed `**` / `__` bold/italic spans (append the matching closer).
  - Unclosed `` ` `` and ``` `` ``` inline code spans.
  - Unclosed `[` link syntax (close with `](...)` or strip the `[`).
  - Unclosed `~~` strikethrough.
  - Does NOT modify already-closed syntax or valid markdown.
- [ ] `StreamingBuffer::flush` applies `heal_markdown` to each stable line before returning it. The raw `tail` is never healed (it's still in progress).
- [ ] `StreamingBuffer::force_flush` also applies `heal_markdown` to the tail before returning (since the stream is ending and we want the final render to be clean).
- [ ] The healed text is used only for rendering — the raw text stored in `ChatMessage.content` and `Part::Text.content` is unhealed. This means `heal_markdown` runs in the `StreamingBuffer` which feeds the render path, not in `stream_response.rs` which feeds the storage path.
- [ ] Existing `StreamingBuffer` tests (fence detection, table detection, debounce) still pass.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `heal_markdown_closes_unclosed_bold` — `heal_markdown("hello **world")` returns `"hello **world**"`.
- [ ] `heal_markdown_closes_unclosed_italic` — `heal_markdown("hello _world")` returns `"hello _world_"`.
- [ ] `heal_markdown_closes_unclosed_inline_code` — `heal_markdown("use `rust")` returns `"use \`rust\`"`.
- [ ] `heal_markdown_closes_unclosed_link` — `heal_markdown("see [docs")` returns `"see [docs](docs)"` or `"see docs"` (strip the `[`).
- [ ] `heal_markdown_leaves_closed_syntax_unchanged` — `heal_markdown("hello **world** and `code`")` returns the input unchanged.
- [ ] `heal_markdown_leaves_plain_text_unchanged` — `heal_markdown("just plain text")` returns `"just plain text"`.
- [ ] `heal_markdown_handles_multiple_unclosed_spans` — `heal_markdown("**bold and `code")` returns `"**bold and `code`**"` (closes both).
- [ ] `streaming_buffer_flush_heals_stable_lines` — push `"hello **world\n"` to the buffer; `flush()` returns `["hello **world**"]` (healed), but the raw tail (if any) is unhealed.
- [ ] `streaming_buffer_force_flush_heals_tail` — push `"hello **world"` (no newline); `force_flush()` returns `["hello **world**"]` (healed because stream is ending).
- [ ] `streaming_buffer_raw_text_not_healed_in_tail` — push `"hello **world\nmore **stuff"`; `flush()` returns `["hello **world**"]` (first line healed), tail is `"more **stuff"` (unhealed, still streaming).

### Layer 2 — Event Handling
- [ ] `append_response_delta_heals_for_display` — feed `ResponseDelta { content: "hello **world\n" }` into `AppState::append_response_delta`; the stable line in the assistant message's display content is `"hello **world**"`. Verify via the `Feed`/`Element` transform that the rendered text shows bold.

### Layer 3 — Rendering
- [ ] `render_healed_bold_shows_styled` — using `TestBackend`, render a message containing a healed `**world**` span; the buffer shows `world` in bold style (not literal `**`).
- [ ] `render_healed_inline_code_shows_styled` — render a message containing healed `` `rust` `` ; the buffer shows `rust` in code style.

### Layer 4 — Smoke / Crash
- [ ] `smoke_heal_markdown_present` — `rg "pub fn heal_markdown" crates/runie-core/src/streaming_buffer.rs` returns a hit.

## Files touched

- `crates/runie-core/src/streaming_buffer.rs` (add `heal_markdown`, apply in `flush`/`force_flush`, ~80 LOC)
- `crates/runie-core/src/markdown/heal.rs` (optional new submodule if `streaming_buffer.rs` would exceed 500 LOC — check current line count)

## Notes

Source inspiration: OpenCode `packages/ui/src/components/markdown-stream.ts:1-49` (uses `remend` library) and Aider `aider/mdstream.py:92-243` (`MarkdownStream` with sliding window). We don't use `remend` (it's a JS library) — a manual span-closing function is ~60 LOC and covers the common cases (`**`, `__`, `` ` ``, `~~`, `[`). The function counts unmatched opener markers and appends the needed closers in reverse order of opening. Don't heal block-level syntax (headings, lists, blockquotes) — those are line-oriented and the `StreamingBuffer`'s stable/tail split already handles them. Healing is display-only: the stored `ChatMessage.content` must remain raw so that when the full text arrives, the final render uses the real syntax, not a healed approximation. This is why `heal_markdown` runs in `StreamingBuffer::flush` (render path) not in `stream_response.rs` (storage path).
