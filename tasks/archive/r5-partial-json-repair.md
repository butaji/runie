# Add partial JSON repair for streaming tool-call arguments

**Status**: done
**Milestone**: R5
**Category**: Tools
**Priority**: P1

**Depends on**: r5-extract-tool-stream
**Blocks**: none

## Description

When tool calls are parsed from plain text fallback (`parse_tool_calls_fallible` in `crates/runie-core/src/tool_parser.rs:36`), incomplete JSON arguments are silently dropped — the parser requires a complete, valid JSON object. During streaming, this means a tool call that arrives near the end of a chunk boundary can be lost if the JSON is truncated. gptme solves this with `find_json_end()` (`gptme/tools/base.py:53-77`), a brace-counting state machine that handles truncated JSON, and Aider's `parse_partial_args()` (`aider/coders/base_coder.py:2338`) tries appending completion suffixes like `]}`, `"}]}`. OpenCode defaults empty argument strings to `"{}"` (`packages/llm/src/protocols/shared.ts:186`). Add a `repair_partial_json` function that tries progressive completion strategies before giving up, and wire it into `ToolStream::finish` (from `r5-extract-tool-stream`) so both structured and text-fallback paths benefit.

## Acceptance Criteria

- [ ] New function `pub fn repair_partial_json(raw: &str) -> Option<Value>` in `crates/runie-core/src/tool_parser.rs` (or a new `tool_parser/json_repair.rs` submodule). It tries, in order:
  1. `serde_json::from_str(raw)` — already valid JSON.
  2. `serde_json::from_str(&(raw.to_string() + "}"))` — missing closing brace.
  3. `serde_json::from_str(&(raw.to_string() + "\"}"))` — missing closing quote and brace.
  4. `serde_json::from_str(&(raw.to_string() + "]"))` — missing closing bracket.
  5. Brace-counting completion: count unmatched `{`, `[`, `"` and append the needed closing chars in reverse order.
  6. Returns `None` if all strategies fail.
- [ ] `ToolStream::finish` (from `r5-extract-tool-stream`) uses `repair_partial_json` instead of `serde_json::from_str` when parsing accumulated arguments.
- [ ] `parse_tool_calls_fallible` uses `repair_partial_json` for the `@tool(id): {json}` and inline-JSON formats.
- [ ] Empty string `""` is treated as `"{}"` (matches OpenCode's default) — `repair_partial_json("")` returns `Some(Value::Object({}))`.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `repair_valid_json_passes_through` — `repair_partial_json(r#"{"command":"ls"}"#)` returns `Some({"command": "ls"})`.
- [ ] `repair_missing_closing_brace` — `repair_partial_json(r#"{"command":"ls""#)` returns `Some({"command": "ls"})`.
- [ ] `repair_missing_closing_quote_and_brace` — `repair_partial_json(r#"{"command":"ls"#)` returns `Some({"command": "ls"})`.
- [ ] `repair_missing_closing_bracket` — `repair_partial_json(r#"{"files":["a","b"#)` returns `Some({"files": ["a", "b"]})`.
- [ ] `repair_empty_string_defaults_to_empty_object` — `repair_partial_json("")` returns `Some({})`.
- [ ] `repair_nested_unclosed` — `repair_partial_json(r#"{"a":{"b":1"#)` returns `Some({"a":{"b":1}})` via brace counting.
- [ ] `repair_garbage_returns_none` — `repair_partial_json("not json at all")` returns `None`.
- [ ] `repair_string_with_escaped_quotes` — `repair_partial_json(r#"{"cmd":"echo \"hi\""#)` returns `Some({"cmd": "echo \"hi\""})` (brace counter tracks escape state).
- [ ] `tool_stream_finish_uses_repair_for_truncated_args` — `ToolStream::start("c1", "bash")`, `append("c1", r#"{"command":"ls"#)` (truncated), `finish("c1")` returns `Some(ParsedToolCall { name: "bash", args: {"command": "ls"} })` instead of `None`.
- [ ] `parse_tool_calls_fallible_recovers_truncated_json` — text containing `@bash(call_1): {"command":"ls` (no closing) still yields a tool call with repaired args.

### Layer 2 — Event Handling
- N/A — pure parsing function; event flow unchanged.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_repair_function_present` — `rg "pub fn repair_partial_json" crates/runie-core/src/tool_parser.rs` returns a hit.

## Files touched

- `crates/runie-core/src/tool_parser.rs` (add `repair_partial_json`, ~60 LOC; update `parse_tool_calls_fallible` to call it, ~5 LOC change)
- `crates/runie-core/src/tool_stream.rs` (from `r5-extract-tool-stream`; use `repair_partial_json` in `finish`, ~3 LOC change)

## Notes

Source inspiration: gptme `gptme/tools/base.py:53-77` (`find_json_end`, brace-counting state machine) and Aider `aider/coders/base_coder.py:2338-2363` (`parse_partial_args`, suffix-appending strategy). The suffix-appending approach is simpler and catches the 90% case (missing `}` or `"}`); the brace-counting approach catches nested structures. We try both. Consider adding the `json_repair` crate (used by gptme) as a dependency if the manual approach proves insufficient — but start manual to avoid a new dep. This task depends on `r5-extract-tool-stream` because `ToolStream::finish` is the single call site for structured tool-call JSON parsing; wiring repair there benefits all providers.
