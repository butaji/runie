# Unify tool-call accumulators into single state machine

## Status

`done`

## Context

The codebase had three distinct tool-call accumulation mechanisms:
1. **`ToolAccumulator`** in `crates/runie-provider/src/protocol.rs` — per-stream accumulator for building tool calls from LLM deltas (OpenAI)
2. **`ToolRegistry`** in `crates/runie-provider/src/protocol.rs` — centralized registry of tool schemas
3. **`ToolStream`** in `crates/runie-core/src/tool_stream.rs` — streaming tool output handler (separate layer)

Analysis showed:
- `ToolAccumulator` in `protocol.rs` is a dead-end: OpenAI has its own `ToolAccum` in `crates/runie-provider/src/openai/`
- `ToolRegistry` in `protocol.rs` is unused: `runie-agent` builds schemas directly from tool impls
- `ToolStream` serves a fundamentally different layer (output streaming) and cannot replace `ToolAccum`

## Changes Made

- Removed `ToolAccumulator` struct and `impl` from `crates/runie-provider/src/protocol.rs`
- Removed `ToolRegistry` struct and `impl` from `crates/runie-provider/src/protocol.rs`
- Removed `HashMap` import from `protocol.rs` (was only used by `ToolRegistry`)
- Removed dead code paths referencing `ToolAccumulator` in `stream_response.rs`
- Removed test for `ToolAccumulator::new` in `protocol.rs`

## Validation

- `cargo test -p runie-provider --lib`: 88 tests pass
- `cargo check --workspace`: passes
- `cargo test --workspace`: 178 passed (7 pre-existing failures in `runie-agent` unrelated to these changes)

## Remaining Work

None. The `ToolStream` layer in `tool_stream.rs` is the correct tool-call streaming mechanism and does not need consolidation.

## Tests

- **Layer 1 — State/Logic:** N/A (no state machine to test after deletion)
- **Layer 2 — Event Handling:** N/A
- **Layer 3 — Rendering:** N/A
- **Layer 4 — E2E:** Existing provider replay tests cover tool call streaming

## Completion Validation

- [x] **Unit tests** — removed dead code had no tests; existing provider tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes (7 pre-existing failures unrelated to this change).
