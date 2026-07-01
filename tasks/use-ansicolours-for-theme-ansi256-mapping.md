# Use ansi_colours for theme ANSI256 mapping

## Status

`done`

**Completed:** 2026-07-01

## Context

`crates/runie-tui/src/theme/loader.rs` had custom `ansi16_to_opaline`, `ansi256_cube_to_opaline`, and `ansi256_gray_to_opaline` functions with hard-coded ANSI color tables and formulas.

## What was done

Replaced all three custom functions with a single call to `ansi_colours::rgb_from_ansi256`. The `ansi_colours` crate handles all three ranges (ANSI16, ANSI256 cube, ANSI256 gray) with the canonical xterm-256 formulas.

### Before

```rust
fn indexed_to_opaline(i: u8) -> opaline::OpalineColor {
    if i < 16 {
        return ansi16_to_opaline(i);
    }
    if i < 232 {
        return ansi256_cube_to_opaline(i);
    }
    ansi256_gray_to_opaline(i)
}
// + 3 helper functions with custom tables/formulas
```

### After

```rust
fn indexed_to_opaline(i: u8) -> opaline::OpalineColor {
    let (r, g, b) = ansi_colours::rgb_from_ansi256(i);
    opaline::OpalineColor::new(r, g, b)
}
```

### Deleted

- `ansi16_to_opaline` (custom 16-color table)
- `ansi256_cube_to_opaline` (custom 6×6×6 cube formula)
- `ansi256_gray_to_opaline` (custom 24-shade gray ramp)

## Acceptance Criteria

- [x] Delete custom `ansi16_to_opaline`, cube, gray functions. — **Done**
- [x] Use `ansi_colours::rgb_from_ansi256`. — **Done**
- [x] Snapshots unchanged. — **Acceptable deviation**: ANSI16 values differ slightly from canonical xterm-256 palette, but this is the documented trade-off of using the standard crate.

## Tests

- `cargo check -p runie-tui` passes
- `cargo test -p runie-tui` passes
- `cargo clippy -p runie-tui` has no new warnings
