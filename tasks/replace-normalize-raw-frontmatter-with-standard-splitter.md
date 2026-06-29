# Replace `normalize_raw_frontmatter` with a standard frontmatter splitter

**Status**: todo
**Milestone**: R7
**Category**: Core / Declarative DSL
**Priority**: P2

**Depends on**: use-pulldown-cmark-frontmatter-for-resource-loader
**Blocks**: none

## Description

`resource_loader::normalize_raw_frontmatter` rewrites raw `---` YAML delimiters into fenced code blocks because `pulldown-cmark-frontmatter` expects fences. Use a frontmatter crate or a thin `serde_yaml`/`toml` splitter that handles standard `---` delimiters directly.

## Acceptance Criteria

- [ ] Delete `normalize_raw_frontmatter`.
- [ ] Frontmatter parsing handles standard `---` delimiters without rewriting.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `frontmatter_parses_without_rewrite` — YAML frontmatter extracted directly.

## Files touched

- `crates/runie-core/src/resource_loader.rs`
- `crates/runie-core/src/declarative/loader.rs`

## Notes

- `matter` or a small regex splitter are acceptable replacements.
