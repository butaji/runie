# Simplify text tool shim parsers

## Status

`done`

## Context

`tool/shim/mod.rs` maintained four fallback parsers for text tool markup (legacy TOOL:, inline JSON, arrow markup, and XML) with three separate branches in `parse_line_strategies` that each had independent error handling.

## Goal

Unify non-XML formats behind a tiny JSON normalizer; keep only the XML path as a distinct shim.

## Implementation

Replaced the three-branch `parse_line_strategies` with a single `try_parse_non_xml_tool` helper that tries formats in order:

1. Inline JSON — `{"name":"bash","arguments":{...}}`
2. `[TOOL_CALL]` markup (arrow syntax) — `{tool => "bash", args => {...}}`
3. Legacy `TOOL:name:arg1:arg2`

Error-returning behavior is preserved: tool-like-but-invalid inputs (malformed JSON, malformed markup) still return `Err` results so callers can surface them to the model.

## Acceptance Criteria

- [x] Each legacy format normalizes to canonical JSON shape.
- [x] Unit tests cover all three non-XML formats (`shim/mod.rs` tests + `runie-agent/src/tests/parser.rs`).
- [x] Mixed-format input (`TOOL:bash + JSON + markup` on separate lines) correctly parses all three tools.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `minimax_m3` — M3 XML format still parses.
- [x] `inline_json` — inline JSON tool call parses.
- [x] `legacy_colon` — `TOOL:bash:ls` colon format parses.
- [x] `markup` — `[TOOL_CALL]` arrow-syntax markup parses.
- [x] `minimax_m2_with_param` — M2 XML with parameters parses.
- [x] `test_parse_markup_malformed_markup_error` — malformed markup returns error.
- [x] `parse_fallible_returns_errors_for_malformed_calls` — malformed JSON returns error.
- [x] `test_parse_markup_mixed_with_other_formats` — all three formats in one input.

### Layer 4 — E2E
- [x] `parse_multiple_tool_calls` — legacy + JSON mixed input.
- [x] `test_parse_read_file_tool`, `test_parse_list_dir_tool`, `test_parse_write_file_tool`, `test_parse_bash_tool` — all legacy formats.
- [x] `headless_runner_feeds_parse_errors_back_to_model` — parse errors propagate to model messages.

### Live tmux tests
- N/A (unit + E2E coverage is sufficient for the parsing layer).

## Files touched

- `crates/runie-core/src/tool/shim/mod.rs` — refactored `parse_line_strategies` into `try_parse_non_xml_tool`; added `TC_START`/`TC_END` guards for error-returning behavior.

## Validation

- [x] **Unit tests** — `cargo test --workspace` passes (all 2957 tests).
- [x] **E2E tests** — provider-replay tests cover mixed-format scenarios.
- [x] **Live tmux run tests** — N/A (parsing layer is covered by unit + E2E tests).
