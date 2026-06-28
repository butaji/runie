# Finish tool-parser shim: marker stripping and MiniMax ownership

**Status**: partial
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

The tool-call parser shim is mostly in place. `crates/runie-core/src/tool/shim/` now owns the text-based fallback parsing, `tool/parse/mod.rs` re-exports from the shim, and `tool/shim/minimax.rs` uses `quick-xml` for MiniMax XML. The remaining work is to collapse the marker-stripping pipeline, reconcile who owns MiniMax XML parsing, and clean up the one compiler warning.

Current state as of this review:

- `quick-xml` is already a direct dependency of `runie-core` (`crates/runie-core/Cargo.toml:40`).
- `crates/runie-core/src/tool/parse/mod.rs` re-exports from `tool::shim`.
- `tool/shim/minimax.rs` uses `quick-xml`.
- `tool/shim/json.rs` implements a single-pass JSON object detector.
- `tool/shim/mod.rs` is ~130 lines and within the build guardrails.
- `tool_markers/strip.rs` still runs an 8-stage pipeline (`strip_tool_call_markup` → `strip_minimax_tool_calls` → `strip_inline_json_objects` → `strip_inline_fenced_tools` → `strip_inline_legacy_tools` → `strip_line_markers` → `strip_empty_code_fences` → `normalize_blank_lines`).
- There is one `cargo check` warning: unused `close_len` in `tool/shim/minimax.rs:47`.
- `docs/Architecture.md:221` says MiniMax XML parsing is isolated in `runie-provider`, but it now lives in `runie-core/src/tool/shim/minimax.rs`.
- The Layer-4 fixtures are Rust constants in `runie-testing::fixtures::minimax`, consumed by `crates/runie-provider/tests/minimax_replay.rs` and `crates/runie-agent/tests/minimax_turn.rs`.
- `cargo test -p runie-agent --lib tests::parser` passes (34 parser tests).

## Acceptance Criteria

- [ ] Fix the unused `close_len` warning in `crates/runie-core/src/tool/shim/minimax.rs`.
- [ ] Shrink `crates/runie-core/src/tool_markers/strip.rs` from the 8-stage pipeline to one or two passes.
- [ ] Keep all currently supported provider/tool output shapes working under the collapsed stripper.
- [ ] Reconcile MiniMax XML parsing ownership with `runie-provider` and `docs/Architecture.md` (either move it all to `runie-provider` or keep the text shim in `runie-core` and document the split).
- [ ] Run the existing MiniMax SSE replay fixtures from `runie-testing::fixtures::minimax`; fix any semantic drift.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `minimax_xml_parsed_by_quick_xml` — feeds representative MiniMax XML tool-call snippets to the shim and asserts the same struct values as the old parser.
- [ ] `json_object_detection_single_pass` — verifies that nested and escaped JSON objects are detected in one pass.
- [ ] `stripper_collapses_to_two_passes` — asserts `strip_all` is implemented as at most two passes and still passes the existing edge-case tests.

### Layer 2 — Event Handling
- [ ] N/A — parsing is pure input transformation.

### Layer 3 — Rendering
- [ ] N/A — this task changes text parsing, not widget output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `text_provider_tool_call_shim` — replays the captured MiniMax SSE fixtures in `runie-testing::fixtures::minimax` through the shim and confirms tool calls, arguments, and markers are stripped identically to the legacy path.

## Files touched

- `crates/runie-core/src/tool/shim/minimax.rs`
- `crates/runie-core/src/tool_markers/strip.rs`
- `crates/runie-core/src/tool_markers/mod.rs`
- `crates/runie-provider/src/` (if MiniMax parsing moves there)
- `docs/Architecture.md` (update parser descriptions)

## Notes

- The parser-shim portion of the original task is complete. This task captures the remaining stripping-pipeline and ownership cleanup.
- `tool_markers/strip.rs` has extensive tests for edge cases (unicode, fenced code, legitimate JSON preservation). Retain those tests while collapsing the implementation.
- Rejected alternative: leaving the 8-stage pipeline in place. It is the remaining complexity hotspot in the text tool boundary.
- Out of scope: adding new tool schemas or changing the MCP boundary itself.
