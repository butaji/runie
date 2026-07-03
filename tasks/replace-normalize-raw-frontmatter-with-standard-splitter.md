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
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
