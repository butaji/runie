# Extract `ToolStream` accumulator module

**Status**: todo
**Milestone**: R5
**Category**: Tools
**Priority**: P0

**Depends on**: none
**Blocks**: r5-provider-protocol-trait, r5-partial-json-repair

## Description

Streaming tool-call accumulation is currently duplicated in two places: `StreamState` in `crates/runie-provider/src/openai/stream.rs:46-50` (a `BTreeMap<usize, Accumulator>` keyed by OpenAI's parallel index) and `StreamState` in `crates/runie-agent/src/stream_response.rs:38-45` (a `HashMap<String, ToolCallAccumulator>` keyed by tool-call id). Both buffer JSON argument fragments across deltas and parse the assembled string on completion. OpenCode centralizes this in a single `ToolStream` module (`packages/llm/src/protocols/utils/tool-stream.ts`) with `start`/`appendOrStart`/`appendExisting`/`finish`/`finishAll`. Extracting the same module in Runie eliminates the duplication, gives the text-fallback parser one place to call, and makes adding new providers cheaper.

## Acceptance Criteria

- [ ] New module `crates/runie-core/src/tool_stream.rs` declares a `ToolStream` struct with `new()`, `start(id, name)`, `append(id, delta_json)`, `finish(id) -> Option<ParsedToolCall>`, `finish_all() -> Vec<ParsedToolCall>`, and `pending() -> impl Iterator<Item = (&String, &Accumulator)>`.
- [ ] Empty argument string defaults to `"{}"` on `finish` (matches OpenCode's `parseToolInput`).
- [ ] `crates/runie-provider/src/openai/stream.rs` replaces its inline `StreamState`/`Accumulator` with `ToolStream`, adapting the OpenAI index-keyed deltas to the id-keyed API (buffering name/id when they arrive after arguments, as the current code already does at lines 231-248).
- [ ] `crates/runie-agent/src/stream_response.rs` replaces its inline `ToolCallAccumulator`/`accumulators` field with `ToolStream`.
- [ ] `crates/runie-core/src/tool_parser.rs` `parse_tool_calls_fallible` path that assembles `@tool(id): {json}` fragments reuses `ToolStream::finish` for JSON parsing.
- [ ] `pub mod tool_stream;` declared in `crates/runie-core/src/lib.rs`.
- [ ] `rg "struct Accumulator\b" crates/` returns zero hits outside `tool_stream.rs`.
- [ ] `rg "struct ToolCallAccumulator\b" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_stream_start_then_append_then_finish` — `start("call_1", "bash")`, `append("call_1", "{\"command\"")`, `append("call_1", ":\"ls\"}")`, `finish("call_1")` yields `ParsedToolCall { name: "bash", args: {"command": "ls"} }`.
- [ ] `tool_stream_finish_empty_args_defaults_to_empty_object` — `start("call_1", "noop")`, `finish("call_1")` yields args `{}` (not an error).
- [ ] `tool_stream_finish_invalid_json_returns_none` — `start("call_1", "bash")`, `append("call_1", "{bad")`, `finish("call_1")` returns `None` (or a `ToolParseError` if the API is widened; keep `Option` for now).
- [ ] `tool_stream_finish_all_drains_pending` — two tools started, only one `finish` called, `finish_all` returns the remaining one.
- [ ] `tool_stream_append_without_start_is_noop` — `append("unknown", "{}")` does not panic; `finish("unknown")` returns `None`.
- [ ] `openai_stream_still_emits_tool_call_start_and_end` — existing tests in `openai/stream.rs` (lines 330-350) still pass unchanged.
- [ ] `stream_response_accumulates_structured_tool_calls` — existing test in `stream_response.rs` (line 203) still passes.

### Layer 2 — Event Handling
- N/A — pure data-structure extraction; event ordering unchanged.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_tool_stream_module_present` — `ls crates/runie-core/src/tool_stream.rs` succeeds; workspace builds.

## Files touched

- `crates/runie-core/src/tool_stream.rs` (new, ~80 LOC)
- `crates/runie-core/src/lib.rs` (add `pub mod tool_stream;`)
- `crates/runie-provider/src/openai/stream.rs` (replace `StreamState`/`Accumulator` with `ToolStream`)
- `crates/runie-agent/src/stream_response.rs` (replace `ToolCallAccumulator`/`accumulators` with `ToolStream`)
- `crates/runie-core/src/tool_parser.rs` (reuse `ToolStream::finish` for JSON parsing in the `@tool(id): {json}` path)

## Notes

Source inspiration: OpenCode `packages/llm/src/protocols/utils/tool-stream.ts` (218 LOC). The Rust version is simpler because we don't need the `route` error-context parameter or the `StreamKey` generic — tool-call ids are `String` everywhere. Keep `ParsedToolCall` (already in `tool_parser.rs`) as the output type so the agent loop doesn't change. Future provider implementations (Anthropic, Ollama) will call `ToolStream::start`/`append`/`finish` directly instead of reimplementing the accumulator.
