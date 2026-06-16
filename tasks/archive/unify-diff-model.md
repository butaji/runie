# Unify Diff Model

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1
**Completed in**: current

**Depends on**: (none)
**Blocks**: (none)

## Description

`runie-agent/src/diff.rs` generates a structured unified diff, then renders
it to a string. `runie-tui/src/diff.rs` parses that string back into a
structured diff to render with colors. The round-trip through text loses
structure and forces two diff models to be maintained.

## Changes Made

Canonical `Diff` type lives in `runie-core/src/diff.rs` with `DiffLine`,
`DiffHunk`, and `Diff` structs. Agent generates it directly via
`Diff::generate()`. TUI renders it via `render_canonical_diff()` without
parsing. String rendering (`to_unified_string()`) is only used for copy/export.
Legacy `parse_diff()` is kept for imperfect agent tool output strings.

## Acceptance Criteria

- [x] A single `Diff` type lives in `runie-core`
- [x] `runie-agent` fills the canonical `Diff` directly
- [x] `runie-tui` renders the canonical `Diff` directly via `render_canonical_diff()`
- [x] String rendering is only used for export/copy, not for the internal pipeline
- [x] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [x] `diff_round_trip_preserves_hunks` — canonical diff survives generation and rendering
- [x] `edit_preview_returns_canonical_diff` — `edit_file` returns the canonical type
- [x] `identical_content_empty_hunks`, `single_line_addition`, `single_line_removal`, `single_line_modification`, `empty_old_content`, `empty_new_content`, `diff_large_file_completes`, `to_unified_string_format`

### Layer 3 — Rendering
- [x] `tui_renders_canonical_diff` — TUI produces styled output from the canonical type

## Files touched

- `crates/runie-core/src/diff.rs` (canonical Diff type)
- `crates/runie-agent/src/diff.rs` (uses runie_core::diff::Diff)
- `crates/runie-tui/src/diff.rs` (render_canonical_diff + legacy parsing)
