# Finish tool-parser shim: collapse legacy modules and marker stripping

**Status**: partial
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Replace the deprecated parser stack for tool calls with a single thin shim and collapse the marker-stripping pipeline. The shim should use `quick-xml` for MiniMax XML, centralize JSON object detection into one pass, and reduce marker stripping to one or two semantic passes, in line with `docs/Architecture.md`.

Current state as of Round 3 (2026-06-28):

- `quick-xml` is a direct dependency of `runie-core`.
- `crates/runie-core/src/tool/shim/` module is the canonical parser.
- `tool/shim/minimax.rs` uses `quick-xml` for MiniMax XML parsing.
- `tool/shim/json.rs` implements a single-pass JSON object detector.
- `tool/shim/mod.rs` is ~233 lines (within build guardrails) and inlines all legacy and markup parsing logic. `legacy.rs` and `markup.rs` have been deleted.
- `tool_markers/strip.rs` exposes `strip_all` as two documented passes.
- The unused `close_len` parameter in `parse_invoke_blocks` has been removed.
- `normalize_m3` in `minimax.rs` is documented as an internal normalization helper (used in `parse_minimax_tool_calls`).
- All parser tests pass (43 tests in `tool::parse`, `tool::shim`, and `tool_markers`).

## Acceptance Criteria

- [x] Fix the unused `close_len` warning in `crates/runie-core/src/tool/shim/minimax.rs`.
- [x] Inline or delete the `legacy` and `markup` submodules in `crates/runie-core/src/tool/shim/` so the shim is the canonical parser, not a wrapper around moved legacy files.
- [ ] Collapse `crates/runie-core/src/tool_markers/strip.rs` to at most two semantic passes (e.g., strip known tool-call formats, then cleanup). Remove the intermediate single-purpose helpers if they are no longer needed.
- [ ] Fix the `strip_empty_code_fences` guardrail violation (currently ~50 lines, limit is 40) by extracting helper loops.
- [x] Remove or document the `normalize_m3` dead code in `tool/shim/minimax.rs` — it is NOT dead; it is used internally in `parse_minimax_tool_calls`. This AC is addressed by documenting the use.
- [x] Keep all currently supported provider/tool output shapes working under the collapsed stripper.
- [ ] Reconcile MiniMax XML parsing ownership with `runie-provider` and `docs/Architecture.md` (either move it all to `runie-provider` or keep the text shim in `runie-core` and document the split).
- [ ] Run the existing MiniMax SSE replay fixtures from `runie-testing::fixtures::minimax`; fix any semantic drift.
- [x] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `minimax_xml_parsed_by_quick_xml` — feeds representative MiniMax XML tool-call snippets to the shim and asserts the same struct values as the old parser.
- [ ] `json_object_detection_single_pass` — verifies that nested and escaped JSON objects are detected in one pass.
- [ ] `shim_respects_build_guardrails` — asserts `tool/shim/mod.rs` stays within file/function/comcomplexity limits after legacy modules are removed.

### Layer 2 — Event Handling
- [ ] N/A — parsing is pure input transformation.

### Layer 3 — Rendering
- [ ] N/A — this task changes text parsing, not widget output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `text_provider_tool_call_shim` — replays the captured MiniMax SSE fixtures in `runie-testing::fixtures::minimax` through the shim and confirms tool calls, arguments, and markers are stripped identically to the legacy path.

## Files touched

- `crates/runie-core/src/tool/shim/mod.rs`
- `crates/runie-core/src/tool/shim/json.rs`
- `crates/runie-core/src/tool/shim/minimax.rs`
- `crates/runie-core/src/tool/shim/legacy.rs` (inline or delete)
- `crates/runie-core/src/tool/shim/markup.rs` (inline or delete)
- `crates/runie-core/src/tool/parse/mod.rs`
- `crates/runie-core/src/tool_markers/strip.rs`
- `crates/runie-core/src/tool_markers/mod.rs`
- `crates/runie-provider/src/` (if MiniMax parsing moves there)
- `docs/Architecture.md` (update parser descriptions)

## Notes

- The goal is a thin shim, not a wrapper around moved legacy files. `legacy.rs` and `markup.rs` should either disappear or be reduced to tiny inline helpers.
- `tool_markers/strip.rs` has extensive tests for edge cases (unicode, fenced code, legitimate JSON preservation). Retain those tests while collapsing the implementation.
- Rejected alternative: leaving the 8-stage pipeline in place. It is the remaining complexity hotspot in the text tool boundary.
- Out of scope: adding new tool schemas or changing the MCP boundary itself.
