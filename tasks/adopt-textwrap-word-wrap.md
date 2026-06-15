# Adopt `textwrap` for Word Wrapping

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace the custom word-wrapping logic in `crates/runie-core/src/layout.rs` and `display_width.rs` with the `textwrap` crate. `textwrap` supports UAX#14 word breaking and optimal-fit wrapping and handles CJK/emoji widths correctly.

## Acceptance Criteria

- [ ] `textwrap` is added as a dependency with `unicode-linebreak` / `unicode-width` features.
- [ ] `layout.rs` / `display_width.rs` wrapping uses `textwrap`.
- [ ] Line counts match current rendered output exactly (scroll math depends on this).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `textwrap_line_count_matches_legacy` — same input produces the same number of wrapped lines.
- [ ] `textwrap_handles_cjk` — CJK text wraps at correct boundaries.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [ ] `wrapped_text_renders_same` — rendered wrapped output matches the legacy renderer.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/layout.rs`
- `crates/runie-core/src/display_width.rs`

## Notes

- This is the riskiest Tier-2 adoption because scroll math depends on exact line counts. Add extensive parity tests before merging.
- See `docs/CRATE_DECISIONS.md`.
