# Delete unused glyph constants from `runie-tui`

**Status**: todo
**Milestone**: R6
**Category": TUI / Rendering
**Priority": P3

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/glyphs.rs` defines many glyphs that are not referenced anywhere in the crate. Move any still-useful ones into `theme/glyph.rs` and delete the file.

## Acceptance Criteria

- [ ] Identify which glyphs are actually used.
- [ ] Move used glyphs to `theme/glyph.rs`.
- [ ] Delete `crates/runie-tui/src/glyphs.rs`.
- [ ] Update imports.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 3 — Rendering
- [ ] `used_glyphs_still_render` — any moved glyphs still appear in expected buffers.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-tui/src/glyphs.rs`
- `crates/runie-tui/src/theme/glyph.rs`
- `crates/runie-tui/src/lib.rs`

## Notes

- Low priority; good cleanup if touching adjacent TUI code.
