# Split tool_parser/mod.rs parsers into submodules

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`tool_parser/mod.rs` has a `parse_tool_calls_fallible` function that is 392 lines with complexity 92, violating both the 40-line and 10-complexity limits. Split the parsing strategies into submodules so the main function is a simple dispatcher.

## Acceptance Criteria

- [x] `parse_tool_calls_fallible` reduced to ≤40 lines with ≤10 complexity.
- [x] Legacy tool parsing moved to `tool_parser/legacy.rs`.
- [x] Inline JSON parsing moved to `tool_parser/inline_json.rs`.
- [x] `[TOOL_CALL]` markup parsing moved to `tool_parser/markup.rs`.
- [x] All helper parsers ≤40 lines each.
- [x] `cargo check --workspace` succeeds with no lint violations.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `parse_legacy_tool_tests` — existing legacy parsing tests still pass.
- [x] `parse_inline_json_tests` — existing inline JSON tests still pass.
- [x] `parse_markup_tests` — existing markup tests still pass.
- [x] `parse_tool_calls_round_trip` — parse + rebuild produces equivalent results.

### Layer 2 — Event Handling
- [x] `parse_tool_calls_handles_all_formats` — all tool call formats parse correctly.

### Layer 3 — Rendering
- N/A — no rendering logic.

### Layer 4 — Smoke / Crash
- [x] `cargo test tool_parser` passes.

## Files touched

- `crates/runie-core/src/tool_parser/mod.rs` — slimmed to dispatcher + helpers.
- `crates/runie-core/src/tool_parser/legacy.rs` — new file for `parse_legacy_tool`.
- `crates/runie-core/src/tool_parser/inline_json.rs` — new file for inline JSON parsing.
- `crates/runie-core/src/tool_parser/markup.rs` — new file for `[TOOL_CALL]` markup.

## Notes

The key insight is that `parse_tool_calls_fallible` is a dispatcher that tries multiple parsing strategies. Each strategy should be its own module with its own helpers. The main function should just iterate and dispatch.
