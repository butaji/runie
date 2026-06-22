# Render TUI diff from canonical core type

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/diff.rs` (466 LOC) defines a parallel diff type hierarchy: `DiffLineType::{Added, Removed, Context, HunkHeader}` (diff.rs:66) is a 1:1 copy of `runie_core::diff::DiffLine`, and `ParsedDiff`/`ParsedDiffLine` (diff.rs:81/89) re-wrap the canonical type. `canonical_to_parsed` (diff.rs:17) is a mechanical match converting core → TUI-private. Both types are used only inside `diff.rs`. Every canonical change forces a parallel TUI enum change.

## Acceptance Criteria

- [x] TUI renders directly from `runie_core::diff::Diff`/`DiffLine`.
- [x] `DiffLineType`, `ParsedDiff`, `ParsedDiffLine`, `canonical_to_parsed` deleted.
- [x] Only the patch-text parser (`parse_diff`) retained for raw tool output.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — type consolidation.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- [x] `render_canonical_diff_unchanged` — TestBackend buffer for a canonical diff is byte-identical before/after refactor.
- [x] `parse_diff_still_parses_unified_diff` — raw patch text still parses.

### Layer 4 — Smoke / Crash
- N/A.

## Files touched

- `crates/runie-tui/src/diff.rs`
- callers of `diff.rs` types in `crates/runie-tui/src/`

## Notes

Keep a snapshot test (insta) of a rendered diff before refactor to assert byte-identity after.
