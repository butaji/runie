# Adopt `syntect` for Syntax Highlighting

**Status**: todo
**Milestone**: R3
**Category**: TUI Rendering
**Priority**: P1

**Depends on**: crate-replacement-audit

## Description

Replace the hand-rolled keyword-based syntax highlighter in
`crates/runie-tui/src/syntax/` with `syntect`. `syntect` uses Sublime Text
syntax definitions and is the standard choice for Rust terminal tools
(`bat`, `delta`, etc.). Context7 ID: `/trishume/syntect`.

## Acceptance Criteria

- [ ] Add `syntect = "5"` to `crates/runie-tui/Cargo.toml`.
- [ ] Delete `crates/runie-tui/src/syntax/` and all its language keyword files.
- [ ] Implement `syntax::highlight_code(code: &str, lang: &str) -> Vec<Line>`
  using `syntect::easy::HighlightLines`.
- [ ] Map `syntect::highlighting::Style` foreground colors to Ratatui
  `Color` (use truecolor when terminal supports it, otherwise nearest 256).
- [ ] Support all languages previously supported (Rust, Python, JS, TS, Go,
  Java, C, C++, SQL, Bash) plus any language with a Sublime grammar.
- [ ] Update `crates/runie-tui/src/markdown.rs` and any callers to use the new
  highlight function.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `highlight_rust_shows_keyword_colors` — `fn`, `let`, `pub` have distinct colors.
- [ ] `highlight_python_shows_def` — `def` keyword colored.
- [ ] `highlight_unknown_language_does_not_panic` — falls back to plain text.

### Layer 3 — Rendering
- [ ] `code_block_renders_with_syntect_colors` — TestBackend shows non-default
  foreground colors in a Rust code block.

## Notes

**ctx7 snippet:**
```rust
let ps = SyntaxSet::load_defaults_newlines();
let ts = ThemeSet::load_defaults();
let syntax = ps.find_syntax_by_extension("rs").unwrap();
let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
```

**Files touched:**
- `crates/runie-tui/Cargo.toml`
- `crates/runie-tui/src/syntax/` (delete)
- `crates/runie-tui/src/lib.rs` (remove module)
- `crates/runie-tui/src/markdown.rs`

**Out of scope:**
- Custom theme loading from `opaline` (can be added later).
