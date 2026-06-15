# Fix Diff Line-Number Parsing

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

`crates/runie-tui/src/diff.rs` loses line numbers for added and removed lines. `parse_added` does `let num = self.new_line_num.take();` and then tries to increment `self.new_line_num`, which is now `None`. The same bug exists in `parse_removed`. As a result, added/removed lines always have `line_number = None`.

## Acceptance Criteria

- [ ] Added lines receive the correct `new_line_num` and the counter is incremented.
- [ ] Removed lines receive the correct `old_line_num` and the counter is incremented.
- [ ] Context lines continue to work as before.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `added_lines_get_numbers` — parsing `+foo` after a hunk header yields `line_number = Some(start)`.
- [ ] `removed_lines_get_numbers` — parsing `-bar` after a hunk header yields `line_number = Some(start)`.

### Layer 2 — Event Handling
N/A — pure parsing.

### Layer 3 — Rendering
- [ ] `diff_renders_line_numbers` — TUI diff widget shows line numbers for added/removed/context lines.

### Layer 4 — Smoke / Crash
N/A — covered by Layer 3.

## Files touched

- `crates/runie-tui/src/diff.rs`

## Notes

- Fix: increment `new_line_num`/`old_line_num` before taking the value in `parse_added`/`parse_removed`, or use `copied()` and increment after.
