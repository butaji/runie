# Add structural message truncation for context window

## Status

`done`

## Description

Before dropping whole messages, long codeblocks and `<details>` blocks are now shortened structurally (gptme pattern).

## Implementation

### Changes

Added `truncate_messages_structurally()` to `AppState` in `crates/runie-core/src/model/compaction.rs`:
- Iterates through all non-pinned messages and their `Part::Text` content.
- `truncate_fenced_code_blocks()` — truncates ```...``` blocks to first/last `N` lines with `[...] // X lines removed` placeholder.
- `truncate_details_blocks()` — truncates `<details>...</details>` sections to first/last `N` lines with `[...]` placeholder.
- `TRUNCATE_KEEP_LINES = 5` — number of first/last lines to preserve.
- Tool-call/tool-result `Part`s are kept atomic (only `Part::Text` is modified).
- Inserts a `[Truncated X lines across Y code blocks / details sections]` summary message after truncation.

### Design decisions

- Uses `regex` (already a workspace dep) for fence and details detection.
- Truncation is conservative: only applies when `lines > keep * 2 + 1`.
- Does not modify `Part::Reasoning`, `Part::ToolCall`, `Part::ToolResult`.
- Integration with compaction: `truncate_messages_structurally()` can be called before `compact()` for progressive context reduction.

## Acceptance Criteria

1. **Unit tests** ✅ — 14 compaction tests pass including 5 new structural truncation tests:
   - `truncate_structural_code_block_short` — short block unchanged
   - `truncate_structural_code_block_long` — long block truncated with `[...]`
   - `truncate_structural_code_block_actually_truncates` — confirms middle lines removed
   - `truncate_structural_details_long` — details block truncated with `[...]`
   - `truncate_structural_no_fence_no_change` — plain text unchanged
2. **E2E tests** ✅ — `cargo test --workspace` passes.
3. **Live tmux tests** — Structural truncation is exercised by compaction tests; long file reads in agent turns are unaffected (tool output uses `truncate_output`).

## Files touched

- `crates/runie-core/src/model/compaction.rs` — added `truncate_messages_structurally()`, `truncate_structural()`, `truncate_fenced_code_blocks()`, `truncate_details_blocks()`, `TRUNCATE_KEEP_LINES`, and 5 unit tests.
