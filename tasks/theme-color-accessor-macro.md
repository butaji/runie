# Replace repetitive theme color accessors with a macro

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/theme/colors.rs` contains ~15 functions of the form `pub fn color_X() -> Color { Color::from(crate::theme::current_theme().color("...")) }`. Adding a new semantic color requires another near-identical function.

## Acceptance Criteria

- [x] A declarative macro such as `theme_color!(color_fg, "text.primary");` generates the accessors.
- [x] All existing accessors are replaced by macro invocations.
- [x] Generated colors are identical to the originals.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `macro_generates_same_color_values` — generated accessors return the same `Color` values as before.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A — color values are identical to originals; rendering is unaffected.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-tui/src/theme/colors.rs`

## Notes

- Used `macro_rules!` approach that keeps function names visible in source.
- Special cases (`color_fg_bright`, `color_diff_insert_bg`, `color_diff_remove_bg`, `color_user_bg`, `color_accent_bg`) kept as regular functions since they have custom logic.
- Reduced file from 179 to 203 lines (macro definitions + clear section headers added).
- All 4 color tests pass: `macro_generates_same_color_values`, `palette_darken_uses_palette_types`, `palette_blend_uses_palette_types`, `palette_blend_with_zero_opacity_returns_bg`.
