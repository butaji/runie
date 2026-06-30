# Extract shared streaming response parser

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: type-and-unify-provider-model-layer
**Blocks**: none

## Description

Every provider reimplements SSE framing, JSON extraction, delta accumulation, and tool-call fragment collection. The shared streaming parser provides a provider-agnostic interface via the `ProviderProtocol` trait that yields a common stream of typed events (text deltas, tool-call start/fragments/finish, errors).

## What was implemented

### Provider Protocol Trait
- `crates/runie-provider/src/protocol.rs` defines `ProviderProtocol` trait
- Abstracts SSE frame parsing and event emission
- State machine handles accumulation and flushing

### OpenAI-Compatible Implementation
- `crates/runie-provider/src/openai/protocol.rs` implements `OpenAiProtocol`
- Handles SSE `data:` lines and JSON parsing
- Accumulates tool call fragments across events
- Supports reasoning/thinking content
- Emits canonical `ProviderEvent`s

### Shared Streaming Utilities
- `crates/runie-provider/src/openai/stream.rs` provides streaming utilities
- `replay_sse()` for testing with captured SSE fixtures
- `parse_sse_event()` for testing individual events
- `collect_events()` test helper

## Acceptance Criteria

- [x] A shared streaming parser exists in `runie-provider`.
- [x] All providers consume the shared parser; duplicated streaming code is deleted.
- [x] Parser handles SSE `data:` lines, partial JSON, and invalid UTF-8 boundaries.
- [x] Tool-call fragments are assembled into complete calls by the parser, not by providers.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `stream_accumulates_tool_call_deltas` — fragmented tool-call JSON is assembled.
- [x] `stream_emits_buffered_arguments_after_delayed_tool_call_id` — delayed id/name handled.
- [x] `openai_stream_accumulates_canonical_tool_calls` — canonical ToolCall assembled.

### Layer 2 — Event Handling
- N/A — parser is a pure stream transform.

### Layer 3 — Rendering
- N/A — parser has no TUI output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `m3_list_files_emits_text_and_json_tool_call` — MiniMax M3 replay fixture.
- [x] `m3_read_file_emits_text_and_json_tool_call` — MiniMax M3 file read fixture.
- [x] `m3_multi_tool_readme_emits_delimited_xml_tool_call` — delimited XML format.
- [x] `m27_multi_tool_readme_emits_standard_xml_tool_call` — standard XML format.
- [x] `openai_streaming_parser` — `openai_stream_accumulates_canonical_tool_calls`.

## Files

- `crates/runie-provider/src/protocol.rs` — `ProviderProtocol` trait
- `crates/runie-provider/src/openai/protocol.rs` — `OpenAiProtocol` implementation
- `crates/runie-provider/src/openai/stream.rs` — streaming utilities and tests
- `crates/runie-provider/tests/minimax_replay.rs` — MiniMax fixture replay tests

## Notes

- The parser operates line-by-line; each complete SSE event is parsed as JSON
- Tool call arguments are accumulated across events via `ToolAccum` state
- OpenAI-compatible providers (MiniMax, Together, etc.) share the same parser
- The `ProviderProtocol` trait allows adding new providers without duplicating streaming logic
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
