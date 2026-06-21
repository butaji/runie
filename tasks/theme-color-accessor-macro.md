# Replace repetitive theme color accessors with a macro

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/theme/colors.rs` contains ~15 functions of the form `pub fn color_X() -> Color { Color::from(crate::theme::current_theme().color("...")) }`. Adding a new semantic color requires another near-identical function.

## Acceptance Criteria

- [ ] A declarative macro such as `theme_color!(color_fg, "text.primary");` generates the accessors.
- [ ] All existing accessors are replaced by macro invocations.
- [ ] Generated colors are identical to the originals.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `macro_generates_same_color_values` — generated accessors return the same `Color` values as before.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] `theme_colors_render_identically` — a widget using generated colors produces the same buffer as before.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-tui/src/theme/colors.rs`

## Notes

If the macro hurts rust-analyzer expansion visibility, consider a `macro_rules` approach that keeps the function names visible in source.
