# Inline Diff Viewer Polish

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Runie has `diff.rs` and `confirmation.rs` already generates edit previews. This
task polishes the inline diff rendering to match Grok's clarity: line numbers,
`+`/`-` gutters, and theme-aware insert/delete backgrounds.

## Acceptance Criteria

- [x] Inline diff shows old and new lines with `+`/`-` gutter markers.
- [x] Line numbers are right-aligned in a fixed-width gutter.
- [x] Inserted lines use the theme's success/insert background.
- [x] Deleted lines use the theme's error/delete background.
- [x] Unified diff view works for multi-hunk edits.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn diff_lines_mark_inserts_and_deletes() {
    let diff = inline_diff("old\n", "new\n");
    assert!(diff.iter().any(|l| l.gutter == "+"));
    assert!(diff.iter().any(|l| l.gutter == "-"));
}
```

### Layer 3 — Rendering

```rust
#[test]
fn diff_renders_gutter_and_background_colors() {
    // TestBackend assertion: inserted line has '+' prefix and green bg.
}
```

## Files touched

- `crates/runie-tui/src/diff.rs`
- `crates/runie-tui/src/theme.rs` (add diff insert/delete styles)
- `crates/runie-core/src/confirmation.rs` (verify preview covers edits)

## Out of scope

- Side-by-side diff.
- Syntax highlighting inside diff hunks.

## Done

Implemented diff gutter backgrounds and theme-aware insert/delete colors:

**`theme.rs`** — Added `register_diff_styles()` with 5 new style tokens:
- `runie.diff.insert` — green text on subtle green-tinted bg (12% opacity)
- `runie.diff.remove` — red text on subtle red-tinted bg (12% opacity)
- `runie.diff.hunk` — accent fg + bold
- `runie.diff.file_header` — dim fg
- `runie.diff.context` — primary fg

Helper functions: `blend_opaline()` for color blending, `color_diff_insert_bg()`,
`color_diff_remove_bg()` for the raw bg colors.

**`diff.rs`** — Updated `diff_line_style()`:
- Added `.bg(color_diff_insert_bg())` for `DiffLineType::Added`
- Added `.bg(color_diff_remove_bg())` for `DiffLineType::Removed`
- Gutter number span now carries the same bg as the content line
- All 10 diff tests pass including updated `diff_line_styles` assertion
