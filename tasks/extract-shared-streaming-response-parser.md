# Extract shared streaming response parser

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: type-and-unify-provider-model-layer
**Blocks**: none

## Description

Every provider currently reimplements SSE framing, JSON extraction, delta accumulation, and tool-call fragment collection. Extract one provider-agnostic streaming parser that yields a common stream of typed events (text deltas, tool-call start/fragments/finish, errors).

## Acceptance Criteria

- [ ] A shared streaming parser exists in `runie-provider`.
- [ ] All providers consume the shared parser; duplicated streaming code is deleted.
- [ ] Parser handles SSE `data:` lines, partial JSON, and invalid UTF-8 boundaries.
- [ ] Tool-call fragments are assembled into complete calls by the parser, not by providers.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `parser_assembles_tool_call` — fragmented tool-call JSON is assembled into one event.
- [ ] `parser_handles_partial_sse_line` — incomplete SSE chunks are buffered until complete.

### Layer 2 — Event Handling
- [ ] N/A — parser is a pure stream transform.

### Layer 3 — Rendering
- [ ] N/A — parser has no TUI output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_m3_streaming_parser` — replay fixture streams through the shared parser and produces correct agent events.
- [ ] `openai_streaming_parser` — second provider fixture exercises the same parser path.

## Files touched

- `crates/runie-provider/src/stream.rs` (new)
- `crates/runie-provider/src/minimax.rs`
- `crates/runie-provider/src/openai.rs`
- `crates/runie-provider/src/anthropic.rs`

## Notes

- Keep the parser synchronous or `async` via `futures::Stream`; do not mix both APIs.
- Use `serde_json::StreamDeserializer` only if it fits the fragment model; otherwise use a manual buffered state machine.
