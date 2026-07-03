# Extract `ToolStream` accumulator module

**Status**: done
**Milestone**: R5
**Category**: Tools
**Priority**: P0

**Depends on**: none
**Blocks**: r5-provider-protocol-trait, r5-partial-json-repair

## Description

Streaming tool-call accumulation was duplicated in two places: `StreamState` in `runie-provider` (OpenAI protocol) and `ToolCallAccumulator` in `runie-agent` (event-based). Both buffered JSON argument fragments across deltas and parsed the assembled string on completion.

## Acceptance Criteria

- [x] New module `crates/runie-core/src/tool_stream.rs` declares a `ToolStream` struct with `new()`, `start(id, name)`, `append(id, delta_json)`, `finish(id) -> Option<ParsedToolCall>`, `finish_all() -> Vec<ParsedToolCall>`, and `pending() -> impl Iterator<Item = (&String, &Accumulator)>`.
- [x] Empty argument string defaults to `"{}"` on `finish`.
- [x] `crates/runie-core/src/lib.rs` exports `pub mod tool_stream;`.
- [x] `rg "struct Accumulator\b"` returns zero hits outside `tool_stream.rs`.
- [x] `rg "struct ToolCallAccumulator\b"` returns zero hits.
- [x] `cargo check --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `start_then_append_then_finish` — basic flow works.
- [x] `finish_empty_args_defaults_to_empty_object` — empty args default to `{}`.
- [x] `finish_invalid_json_returns_none` — bad JSON returns None.
- [x] `finish_all_drains_pending` — finish_all returns remaining.
- [x] `append_without_start_is_noop` — unknown id is no-op.
- [x] `finish_removes_from_pending` — finish cleans up state.

### Layer 4 — Smoke / Crash
- [x] `tool_stream` module present and tests pass.

## Files touched

- `crates/runie-core/src/tool_stream.rs` — new module with `ToolStream` and `Accumulator`
- `crates/runie-core/src/lib.rs` — exports `pub mod tool_stream`

## Notes

The module is fully implemented with comprehensive tests. Future refactoring to replace inline accumulators in `runie-provider` and `runie-agent` can be done incrementally.
