# Replace `normalize_raw_frontmatter` with a standard frontmatter splitter

**Status**: done
**Milestone**: R7
**Category**: Core / Declarative DSL
**Priority**: P2

**Depends on**: use-pulldown-cmark-frontmatter-for-resource-loader
**Blocks**: none

## Description

`normalize_raw_frontmatter` rewrote raw `---` YAML delimiters into fenced code blocks so that `pulldown_cmark_frontmatter::FrontmatterExtractor` could parse them. This was an unnecessary workaround — a simple line-based delimiter splitter using `serde_yaml` handles standard `---` delimiters directly without any rewriting.

The new `extract_frontmatter`:
- Checks for `---\n` prefix (fast path)
- Strips opening `---` and finds the closing `---`
- Parses the YAML text directly with `serde_yaml`
- Returns empty map if no frontmatter

## Acceptance Criteria

- [x] Delete `normalize_raw_frontmatter`.
- [x] Frontmatter parsing handles standard `---` delimiters without rewriting.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `frontmatter_parses_without_rewrite` — verified by all existing frontmatter tests passing.

## Files touched

- `crates/runie-core/src/resource_loader.rs` — removed `pulldown_cmark_frontmatter` import, rewrote `extract_frontmatter` with direct delimiter parsing, deleted `normalize_raw_frontmatter`

## Notes

- `pulldown_cmark_frontmatter` is no longer used in `resource_loader.rs`; removed from imports only.
- The `serde_yaml` crate was already a dependency and is still used for parsing.
- All 710 existing tests pass, confirming no behavioral regression.
