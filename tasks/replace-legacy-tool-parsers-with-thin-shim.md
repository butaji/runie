# Replace legacy tool parsers with a thin shim

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: make-mcp-the-only-tool-boundary
**Blocks**: none

## Description

Replace the deprecated parser stack for tool calls (`legacy`, `json`, `markup`, and `minimax` parsers) and the seven-stage stripping pipeline in `tool_markers/strip.rs` with a single thin shim. The shim should use `quick-xml` for MiniMax XML, centralize JSON object detection into one pass, and reduce marker stripping to one or two passes, in line with `docs/Architecture.md`.

To keep the build and provider-replay tests stable, introduce the new shim **alongside** the legacy parsers first, route all callers through it, run the existing Layer-4 fixtures, and only then delete the legacy files.

## Acceptance Criteria

- [ ] Add `quick-xml` as a direct dependency of `crates/runie-core` (it is currently only transitive via `Cargo.lock`).
- [ ] Create `crates/runie-core/src/tool/shim.rs` (or equivalent) implementing the thin shim using `quick-xml` for MiniMax XML and a single JSON-object detector.
- [ ] Route `strip_tool_markers`, `parse_tool_calls_fallible`, and all parser entry points through the shim without changing their public signatures.
- [ ] Run the existing MiniMax SSE replay fixtures and parser regression tests; fix any semantic drift.
- [ ] Delete `crates/runie-core/src/tool/parse/legacy.rs`, `json.rs`, `markup.rs`, and `minimax.rs`.
- [ ] Shrink `crates/runie-core/src/tool_markers/strip.rs` from a seven-stage pipeline to one or two passes.
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
- [ ] `text_provider_tool_call_shim` — replays the captured MiniMax SSE fixtures in `crates/runie-provider/tests/fixtures/minimax/` and `crates/runie-agent/tests/fixtures/minimax/` through the new shim and confirms tool calls, arguments, and markers are stripped identically to the legacy path.

## Files touched

- `crates/runie-core/src/tool/parse/legacy.rs` (delete)
- `crates/runie-core/src/tool/parse/json.rs` (delete)
- `crates/runie-core/src/tool/parse/markup.rs` (delete)
- `crates/runie-core/src/tool/parse/minimax.rs` (delete)
- `crates/runie-core/src/tool/parse/mod.rs`
- `crates/runie-core/src/tool/shim.rs` (new)
- `crates/runie-core/src/tool_markers/strip.rs`
- `crates/runie-core/src/tool_markers/mod.rs`
- `crates/runie-core/Cargo.toml` (add `quick-xml` dependency if missing)
- `docs/Architecture.md` (update if parser descriptions are now stale)

## Notes

`docs/Architecture.md` already describes this layer as a thin shim rather than a permanent parser stack. The legacy parsers were retained for backwards compatibility during the MCP transition; once `make-mcp-the-only-tool-boundary` lands they are no longer needed. Rejected alternative: refactoring the legacy parsers in place — the parser-specific state machines are the complexity we want to remove. Out of scope: adding new tool schemas or changing the MCP boundary itself.
