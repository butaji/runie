# Inline Diff Viewer Polish

**Status**: todo
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

- [ ] Inline diff shows old and new lines with `+`/`-` gutter markers.
- [ ] Line numbers are right-aligned in a fixed-width gutter.
- [ ] Inserted lines use the theme's success/insert background.
- [ ] Deleted lines use the theme's error/delete background.
- [ ] Unified diff view works for multi-hunk edits.

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
