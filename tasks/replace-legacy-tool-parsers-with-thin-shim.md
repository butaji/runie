# Replace legacy tool parsers with a thin shim

**Status**: partial
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Replace the deprecated parser stack for tool calls (`legacy`, `json`, `markup`, and `minimax` parsers) and the multi-stage stripping pipeline in `tool_markers/strip.rs` with a single thin shim. The shim should use `quick-xml` for MiniMax XML, centralize JSON object detection into one pass, and reduce marker stripping to one or two passes, in line with `docs/Architecture.md`.

Current state as of this review:

- `quick-xml` is already a direct dependency of `runie-core` (`crates/runie-core/Cargo.toml:40`).
- A new `crates/runie-core/src/tool/shim/` module exists and `crates/runie-core/src/tool/parse/mod.rs` now re-exports from it.
- `tool/shim/minimax.rs` already uses `quick-xml` for MiniMax XML parsing.
- `tool/shim/json.rs` already implements a single-pass JSON object detector.
- **Still incomplete:** `tool/shim/mod.rs` is 571 lines (exceeds the 500-line build guardrail) and still embeds `legacy`, `markup`, and `minimax` submodules instead of replacing them.
- `tool_markers/strip.rs` still runs the original multi-stage pipeline.
- The legacy files have been removed from `tool/parse/` but still exist inside `tool/shim/`.
- The Layer-4 fixtures named in the original task (`crates/runie-provider/tests/fixtures/minimax/`, `crates/runie-agent/tests/fixtures/minimax/`) do not exist. Fixtures are Rust constants in `runie-testing::fixtures::minimax` and are consumed by `crates/runie-provider/tests/minimax_replay.rs` and `crates/runie-agent/tests/minimax_turn.rs`.
- `docs/Architecture.md:221` says MiniMax XML parsing is isolated in `runie-provider`, but `runie-core/src/tool/shim/minimax.rs` also parses MiniMax XML. This split must be reconciled before a single shim is canonical.
- As of this review, `cargo test --workspace` fails with 2 parser regressions in `runie-agent`: `tests::parser::parse_minimax_list_dir_tool` (argument value drifts from `"."` to `Null`) and `tests::parser::test_parse_markup_malformed_markup_error` (returns 0 results instead of 1 error). These must pass before the task is complete.

## Acceptance Criteria

- [ ] Collapse `tool/shim/mod.rs` so it respects the 500-line file limit and the complexity guardrail (no function > 10).
- [ ] Remove the embedded `legacy`, `markup`, and `minimax` submodules from `tool/shim/`; the shim should either implement their behavior directly or delete the obsolete formats.
- [ ] Route `strip_tool_markers`, `parse_tool_calls_fallible`, and all parser entry points through the shim without changing their public signatures.
- [ ] Shrink `crates/runie-core/src/tool_markers/strip.rs` from a multi-stage pipeline to one or two passes.
- [ ] Reconcile MiniMax XML parsing ownership with `runie-provider` and `docs/Architecture.md` (either move it all to `runie-provider` or keep the text shim in `runie-core` and document the split).
- [ ] Run the existing MiniMax SSE replay fixtures from `runie-testing::fixtures::minimax`; fix any semantic drift.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `minimax_xml_parsed_by_quick_xml` — feeds representative MiniMax XML tool-call snippets to the shim and asserts the same struct values as the old parser.
- [ ] `json_object_detection_single_pass` — verifies that nested and escaped JSON objects are detected in one pass without re-running the legacy regex stack.
- [ ] `shim_respects_build_guardrails` — asserts `tool/shim/mod.rs` is ≤ 500 lines and no shim function exceeds complexity 10.

### Layer 2 — Event Handling
- [ ] N/A — parsing is pure input transformation; no crossterm-style events are handled.

### Layer 3 — Rendering
- [ ] N/A — this task changes text parsing, not widget output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `text_provider_tool_call_shim` — replays the captured MiniMax SSE fixtures in `runie-testing::fixtures::minimax` through the new shim and confirms tool calls, arguments, and markers are stripped identically to the legacy path.
- [ ] `parse_minimax_list_dir_tool` — the existing `runie-agent` test for MiniMax `list_dir` parsing passes (argument value `"."` is preserved, not `Null`).
- [ ] `test_parse_markup_malformed_markup_error` — the existing `runie-agent` test for malformed markup returns one parse error, not zero results.

## Files touched

- `crates/runie-core/src/tool/shim/mod.rs` (collapse / fix guardrails)
- `crates/runie-core/src/tool/shim/json.rs`
- `crates/runie-core/src/tool/shim/minimax.rs`
- `crates/runie-core/src/tool/shim/legacy.rs` (delete or inline)
- `crates/runie-core/src/tool/shim/markup.rs` (delete or inline)
- `crates/runie-core/src/tool/parse/mod.rs`
- `crates/runie-core/src/tool_markers/strip.rs`
- `crates/runie-core/src/tool_markers/mod.rs`
- `crates/runie-provider/src/` (if MiniMax parsing moves there)
- `docs/Architecture.md` (update parser descriptions)

## Notes

- The workspace currently fails `cargo check --workspace` because the partial shim violates the build guardrails. Fixing those violations is part of this task.
- `docs/Architecture.md` already describes this layer as a thin shim rather than a permanent parser stack. The legacy parsers were retained for backwards compatibility during the MCP transition; once MCP is the only tool boundary they are no longer needed.
- Rejected alternative: refactoring the legacy parsers in place — the parser-specific state machines are the complexity we want to remove.
- Out of scope: adding new tool schemas or changing the MCP boundary itself.
