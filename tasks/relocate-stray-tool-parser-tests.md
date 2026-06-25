# Relocate stray top-level tool_parser_tests.rs

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: consolidate-dual-path-modules
**Blocks**: none

## Description

`crates/runie-core/src/tool_parser_tests.rs` (106 LOC) is a stray top-level test file. The module it tests (`tool_parser`) lives at `tool_parser/mod.rs` after `consolidate-dual-path-modules` lands. The workspace convention elsewhere is `foo/tests.rs` co-located with the module (e.g. `keybindings/tests.rs`, `login_config/tests.rs`, `model/cache/tests.rs`). The top-level `<name>_tests.rs` form is unique to this one file and breaks `rg`/glob discoverability.

## Acceptance Criteria

- [ ] `crates/runie-core/src/tool_parser_tests.rs` deleted.
- [ ] Contents moved to `crates/runie-core/src/tool_parser/tests.rs` (or inlined into `tool_parser/mod.rs` behind `#[cfg(test)] mod tests`).
- [ ] `tool_parser/mod.rs` declares `#[cfg(test)] mod tests;` (or the inline block).
- [ ] No `tool_parser_tests.rs` remains anywhere in the workspace.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace -p runie-core --lib tool_parser` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `relocated_tests_still_parse_tool_calls` — the moved test cases still exercise `parse_tool_calls` / `assign_tool_call_ids` and pass.

### Layer 2 — Event Handling
- N/A — test relocation, no event flow.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_tool_parser_test_module_compiles` — `cargo test -p runie-core --lib tool_parser` compiles and runs the relocated tests.

## Files touched

- `crates/runie-core/src/tool_parser_tests.rs` (deleted)
- `crates/runie-core/src/tool_parser/mod.rs` (add `mod tests;`)

## Notes

Run after `consolidate-dual-path-modules` so the destination is `tool_parser/tests.rs` next to `tool_parser/mod.rs`. If the consolidate task stalls, this can land first by inlining the tests behind `#[cfg(test)] mod tests` directly in `tool_parser.rs`.
