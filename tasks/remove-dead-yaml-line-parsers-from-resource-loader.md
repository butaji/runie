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
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
