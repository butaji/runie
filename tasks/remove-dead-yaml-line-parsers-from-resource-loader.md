# Remove dead YAML line parsers from `resource_loader`

**Status**: done
**Note**: Verified 2026-06-29 — `parse_yaml_line` and `strip_quotes` not found in codebase.
**Milestone**: R7
**Category**: Core / Declarative DSL
**Priority**: P2

**Depends on**: use-pulldown-cmark-frontmatter-for-resource-loader
**Blocks**: none

## Description

`parse_yaml_line` and `strip_quotes` in `crates/runie-core/src/resource_loader.rs` are only used in tests. The declarative loaders deserialize YAML through `serde_yaml`. Delete the dead helpers and their tests.

## Acceptance Criteria

- [x] Delete `parse_yaml_line` and `strip_quotes`.
- [x] Delete their tests.
- [x] Remove exports from `lib.rs` if present.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `resource_loader_has_no_yaml_line_helpers` — grep confirms deletion.

## Files touched

- `crates/runie-core/src/resource_loader.rs`
- `crates/runie-core/src/lib.rs`
- `crates/runie-core/src/declarative/tests.rs`

## Notes

- Low risk; these functions are dead in production.
