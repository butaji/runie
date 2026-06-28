# Unify markdown processing around `pulldown-cmark`

**Status**: todo
**Milestone**: R1
**Category**: Core / State
**Priority**: P0

**Depends on**: replace-legacy-tool-parsers-with-thin-shim, use-pulldown-cmark-for-tool-marker-stripping
**Blocks**: none

## Description

Runie currently maintains three markdown-aware pipelines that disagree on fences, tables, and inline spans:
- `crates/runie-core/src/markdown/` — uses `pulldown-cmark` and provides block/inline AST used by `layout.rs`.
- `crates/runie-core/src/streaming_buffer.rs` — re-implements its own fence/table classifier and a second `heal_markdown` (~381 LOC).
- `crates/runie-core/src/markdown/heal.rs` — a third `heal_markdown` implementation that is not exported or used anywhere.
- `crates/runie-core/src/tool_markers/strip.rs` — regex-based stripping (already planned for `pulldown-cmark`).

The `markdown/` module should become the single authority. `StreamingBuffer` should use it for stable-line rendering and fence detection; `markdown/heal.rs` should be deleted or merged; and tool-marker stripping should walk `pulldown-cmark` events.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/markdown/heal.rs` unless it is genuinely used (current grep shows no callers).
- [ ] Rewrite `streaming_buffer.rs` to delegate fence/table/inline-span detection to `markdown::parse_markdown` (or equivalent) instead of its custom char-stack parser.
- [ ] Complete `use-pulldown-cmark-for-tool-marker-stripping` so the stripper is also a `pulldown-cmark` event pass.
- [ ] Ensure `layout.rs` line counts and TUI rendering still agree (preserve Layer-3 agreement tests).
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `heal_markdown_deleted` — `markdown/heal.rs` no longer exists.
- [ ] `streaming_buffer_uses_markdown_module` — `streaming_buffer.rs` imports from `crate::markdown` and contains no custom fence classifier.

### Layer 3 — Rendering
- [ ] `layout_and_tui_agree_on_line_counts` — existing agreement tests in `runie-tui/src/ui/messages/lines.rs` still pass.
- [ ] `stripper_produces_clean_markdown` — tool-marker stripping still passes its fixtures.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `provider_replays_render_after_markdown_unification` — MiniMax and other provider replays produce the same rendered output.

## Files touched

- `crates/runie-core/src/markdown/heal.rs`
- `crates/runie-core/src/streaming_buffer.rs`
- `crates/runie-core/src/markdown/mod.rs`
- `crates/runie-core/src/markdown/blocks.rs`
- `crates/runie-core/src/markdown/inline.rs`
- `crates/runie-core/src/tool_markers/strip.rs`
- `crates/runie-core/src/layout.rs`

## Notes

- This task depends on the tool-parser shim being canonical; do not start before `replace-legacy-tool-parsers-with-thin-shim` lands.
- `goose` and `jcode` route all markdown processing through `pulldown-cmark`; Runie should too.
- The `markdown/` module may need to expose incremental/line-oriented entry points so the render path can ask "is this line stable?" without parsing the whole stream.
