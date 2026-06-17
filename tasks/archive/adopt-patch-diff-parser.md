# Adopt `patch` Crate for TUI Diff Parsing

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace the hand-rolled unified-diff parser in `crates/runie-tui/src/diff.rs` with the `patch` crate. The `patch` crate understands `---`, `+++`, `@@`, and `+`/`-`/context lines and returns structured hunks. Runie’s Ratatui renderer and gutter/colors stay; only parsing changes.

## Acceptance Criteria

- [ ] `patch` crate is added as a dependency to `runie-tui`.
- [ ] `diff.rs` uses `patch::Patch::from_single` (or equivalent) to parse diff strings.
- [ ] Line numbers are correctly populated for added/removed/context lines.
- [ ] Rendering fallback for non-diff text is preserved.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `patch_parses_unified_diff` — a sample diff parses into hunks with correct line numbers.
- [ ] `patch_handles_single_file_diff` — `Patch::from_single` works on agent-produced diffs.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [ ] `diff_renders_parsed_hunks` — TUI diff widget renders parsed hunks with colors and gutters.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-tui/Cargo.toml`
- `crates/runie-tui/src/diff.rs`

## Notes

- The `similar` crate is already used in `runie-agent` for diff generation; this task is for parsing/rendering diffs in the TUI.
- See `docs/CRATE_DECISIONS.md`.
