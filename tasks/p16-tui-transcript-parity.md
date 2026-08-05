# p16 — TUI: transcript rendering parity (verb-group activity folding, markdown, tool cards, reasoning fold)

**Parity target:** grok scrollback rendering.

## Grok reference

`~/Code/agents/grok-build/crates/codegen/xai-grok-pager/src/scrollback/render.rs`
- **Verb-group activity folding**: consecutive tool calls of the same verb fold into a header row — `"Read 2 files"`, `"Listed 1 dir"`, `"Ran 1 subagent"`, combined `"Read 1 file, Ran 1 subagent"` (render.rs:1508,1628,1743,1802). Folding keeps a live ` — activity` suffix while running (render.rs:1757-1759).
- Tool cards: name + args + spinner while running, success `✓`/error `✗` marker, structured output rows (from `tool_execution_*` events).
- Reasoning: dim/italic transcript style, collapsible (fold closed/open).
- Markdown: grok uses the `xai-grok-markdown` renderer (`crates/codegen/xai-grok-markdown`) for bold, bullets, headings, code, links.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-tui/src/widgets/scrollback.rs` + `event_renderer.rs`
- Has `Line`/`LineKind` (User, Assistant, Tool, ToolOutput, Activity, Reasoning, System), reasoning fold, tool cards, activity grouping (`"◈ Listed 1 dir, Read 1 file"`), markdown bold/bullets (via `markdown_spans`).

## Adapt to runie

1. **Verb-group folding parity**: verify the activity line format matches grok's `"Read 2 files"` / `"Listed 1 dir"` verb-group headers (runie currently renders a combined `"Listed N dir, Read N files"`). Adjust to grok's folded-header + live-activity layout and add collapsible member rows for grouped tool outputs.
2. **Markdown completeness**: add code blocks, fenced code, headings, links, inline code, lists to the markdown renderer (align with `xai-grok-markdown`). Currently bold + bullets only.
3. **Tool cards**: render name + args + spinner + `✓`/`✗` + structured output rows from `ToolExecutionStart/Update/End` (runie has basic version; verify error marker and update lines match grok).
4. **Reasoning fold**: dim/italic collapsed/expanded cells matching grok transcript style (runie has this; verify glyphs/style).
5. **Gutter/cursor**: user feed cursor at column five, blue pointer without bold body (from earlier visual work — retain).

## State machine / variants

Transcript block states:
- `ToolBlock`: `running` (spinner + live activity) → `success(✓)` | `error(✗)`; then `collapsed` (header) | `expanded` (member rows).
- `ReasoningBlock`: `collapsed` | `expanded` (dim/italic).
- `ActivityGroup`: `running(label)` → `idle(label)`.
- `LineKind` variants (already): `User | Assistant | Tool | ToolOutput | Activity | Reasoning | System`.

## Acceptance

- Extend the visual snapshot suite (`visual_snapshots.rs`) with: verb-group folding header, tool error marker, code-block markdown, expanded reasoning cells.
- Compare transcript rows against the recorded grok casts (`artifacts/grok-full.cast`, `grok-rich.cast`) with zero diffs (see p19).
- `cargo test -p runie-tui` green.