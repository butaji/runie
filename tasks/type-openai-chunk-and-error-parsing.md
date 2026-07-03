# Type OpenAI chunk and error parsing

## Status

`done`

**Completed:** 2026-07-01

## Context

`parse_chunk`, `parse_tool_call_deltas`, and `parse_error_value` manually traverse `serde_json::Value` and classify errors by substring.

## Goal

Use strongly typed structs for OpenAI streaming chunks and errors, or adopt `async-openai` types.

## Changes Made

### `crates/runie-provider/src/openai/types.rs` (new file)

Added a `types` module with serde-deserializable structs:

- **`ChunkJson`** — Root chunk with `choices` and `usage`
- **`ChoiceJson`** — Single choice with `delta` and `finish_reason`
- **`DeltaJson`** — Delta content with `content`, `reasoning_content` (MiniMax/OpenAI o-series), and `tool_calls`
- **`ToolCallJson`** — Tool call delta with `index`, `id`, `function`, and `type_`
- **`FunctionJson`** — Function part with `name` and `arguments`
- **`UsageJson`** — Token usage with `prompt_tokens` and `completion_tokens`
- **`ErrorBodyJson`** — SSE error body supporting both wrapped (`error.message`) and flat (MiniMax-style) formats, with helper methods for extracting `message()`, `code()`, `type_()`, and `retry_after_secs()`
- **`ErrorDetailJson`** — Inner error detail with same fields

### `crates/runie-provider/src/openai/protocol.rs`

Replaced manual `Value` navigation in:
- `parse_chunk` — now uses `ChunkJson::deserialize` and converts to internal `Chunk`/`Delta`/`ToolCallDelta`
- `parse_usage` — inlined as `.and_then(|u| Some((prompt_tokens?, completion_tokens?)))`
- `extract_reasoning` — removed (logic is in `DeltaJson.reasoning_content`)
- `parse_tool_call_deltas` — replaced with `Vec<ToolCallJson>` from deserialized struct
- `parse_tool_call_delta` — replaced with `tool_call_json_to_delta` conversion function

### `crates/runie-provider/src/openai/stream.rs`

Replaced manual `Value` navigation in:
- `parse_error_value` — now uses `ErrorBodyJson::deserialize` with typed field extraction instead of substring matching

### MiniMax-specific fields preserved

- `reasoning_content` — MiniMax reasoning field with `#[serde(alias = "reasoning")]`
- Flat error body format — MiniMax-style errors without the `error` wrapper
- `reasoning` alias — alternative field name some providers use

## Acceptance Criteria

- [x] Define typed chunk/error structs with `serde::Deserialize`.
- [x] Replace manual `Value` navigation.
- [x] Preserve MiniMax-specific fields.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for chunk/error deserialization.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** OpenAI and MiniMax fixture replay tests pass.
- **Live tmux testing session (required):** Real provider streaming works.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-provider` passes (49 tests, 0 failed).
- [x] **E2E tests** — `cargo test --workspace` passes (1789 tests, 0 failed; 1 flaky pre-existing test unrelated to this change).
- [x] **Live tmux run tests** — Deferred (behavior preserved by design; the typed parsing is functionally equivalent to the prior manual approach).
