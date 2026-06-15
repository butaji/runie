# Unify Diff Model

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

`runie-agent/src/diff.rs` generates a structured unified diff, then renders
it to a string. `runie-tui/src/diff.rs` parses that string back into a
structured diff to render with colors. The round-trip through text loses
structure and forces two diff models to be maintained.

## Acceptance Criteria

- [ ] A single `Diff` type lives in `runie-core` (or `runie-tui` if it is
  purely a rendering concern).
- [ ] `runie-agent` fills the canonical `Diff` directly and no longer
  renders to string internally.
- [ ] `runie-tui` renders the canonical `Diff` directly and no longer parses
  a string.
- [ ] String rendering is only used for export/copy, not for the internal
  pipeline.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `diff_round_trip_preserves_hunks` — canonical diff survives
  generation and rendering without loss.
- [ ] `edit_file_returns_canonical_diff` — `edit_file` returns the canonical
  type.

### Layer 3 — Rendering
- [ ] `tui_renders_canonical_diff` — TUI produces the same styled output
  from the canonical type as it did from parsed text.

## Files touched

- `crates/runie-core/src/diff.rs` (new)
- `crates/runie-agent/src/diff.rs`
- `crates/runie-tui/src/diff.rs`
- `crates/runie-core/src/edit_preview.rs`
- Call sites that display diffs.

## Notes

`similar::TextDiff` can still be used under the hood; the unification is
about the data type that crosses the agent→TUI boundary.
