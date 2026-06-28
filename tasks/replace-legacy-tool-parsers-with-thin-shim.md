# Replace legacy tool parsers with a thin shim

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Replace the deprecated parser stack for tool calls (`legacy`, `json`, `markup`, and `minimax` parsers) and the multi-stage stripping pipeline in `tool_markers/strip.rs` with a single thin shim. The shim should use `quick-xml` for MiniMax XML, centralize JSON object detection into one pass, and reduce marker stripping to one or two passes, in line with `docs/Architecture.md`.

Current state as of this review:

- `quick-xml` is already a direct dependency of `runie-core` (`crates/runie-core/Cargo.toml:40`).
- The legacy parsers still live in `crates/runie-core/src/tool/parse/{legacy,json,markup,minimax}.rs`.
- `parse_tool_calls_fallible` (`tool/parse/mod.rs:33–47`) routes through the legacy parsers.
- `tool_markers/strip.rs` runs a multi-stage pipeline (`strip_tool_call_markup` → `strip_minimax_tool_calls` → `strip_inline_json_objects` → `strip_inline_fenced_tools` → `strip_inline_legacy_tools` → `strip_line_markers` → `strip_empty_code_fences` → `normalize_blank_lines`).
- The Layer-4 fixtures named in the original task (`crates/runie-provider/tests/fixtures/minimax/`, `crates/runie-agent/tests/fixtures/minimax/`) do not exist. Fixtures are Rust constants in `runie-testing::fixtures::minimax` and are consumed by `crates/runie-provider/tests/minimax_replay.rs` and `crates/runie-agent/tests/minimax_turn.rs`.
- `docs/Architecture.md:221` says MiniMax XML parsing is isolated in `runie-provider`, but `runie-core/src/tool/parse/minimax.rs` also parses MiniMax XML. This split must be reconciled before a single shim is canonical.

To keep the build and provider-replay tests stable, introduce the new shim **alongside** the legacy parsers first, route all callers through it, run the existing Layer-4 fixtures, and only then delete the legacy files.

## Acceptance Criteria

- [ ] Create `crates/runie-core/src/tool/shim.rs` (or equivalent) implementing the thin shim using `quick-xml` for MiniMax XML and a single JSON-object detector.
- [ ] Route `strip_tool_markers`, `parse_tool_calls_fallible`, and all parser entry points through the shim without changing their public signatures.
- [ ] Reconcile MiniMax XML parsing ownership with `runie-provider` and `docs/Architecture.md` (either move it all to `runie-provider` or keep the text shim in `runie-core` and document the split).
- [ ] Run the existing MiniMax SSE replay fixtures from `runie-testing::fixtures::minimax`; fix any semantic drift.
- [ ] Delete `crates/runie-core/src/tool/parse/legacy.rs`, `json.rs`, `markup.rs`, and `minimax.rs`.
- [ ] Shrink `crates/runie-core/src/tool_markers/strip.rs` from a multi-stage pipeline to one or two passes.
- [ ] Keep all currently supported provider/tool output shapes working under the shim.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `minimax_xml_parsed_by_quick_xml` — feeds representative MiniMax XML tool-call snippets to the shim and asserts the same struct values as the old parser.
- [ ] `json_object_detection_single_pass` — verifies that nested and escaped JSON objects are detected in one pass without re-running the legacy regex stack.

### Layer 2 — Event Handling
- [ ] N/A — parsing is pure input transformation; no crossterm-style events are handled.

### Layer 3 — Rendering
- [ ] N/A — this task changes text parsing, not widget output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `text_provider_tool_call_shim` — replays the captured MiniMax SSE fixtures in `runie-testing::fixtures::minimax` through the new shim and confirms tool calls, arguments, and markers are stripped identically to the legacy path.

## Files touched

- `crates/runie-core/src/tool/parse/legacy.rs` (delete)
- `crates/runie-core/src/tool/parse/json.rs` (delete)
- `crates/runie-core/src/tool/parse/markup.rs` (delete)
- `crates/runie-core/src/tool/parse/minimax.rs` (delete)
- `crates/runie-core/src/tool/parse/mod.rs`
- `crates/runie-core/src/tool/shim.rs` (new)
- `crates/runie-core/src/tool_markers/strip.rs`
- `crates/runie-core/src/tool_markers/mod.rs`
- `crates/runie-provider/src/` (if MiniMax parsing moves there)
- `docs/Architecture.md` (update parser descriptions)

## Notes

- `docs/Architecture.md` already describes this layer as a thin shim rather than a permanent parser stack. The legacy parsers were retained for backwards compatibility during the MCP transition; once MCP is the only tool boundary they are no longer needed.
- Rejected alternative: refactoring the legacy parsers in place — the parser-specific state machines are the complexity we want to remove.
- Out of scope: adding new tool schemas or changing the MCP boundary itself.
