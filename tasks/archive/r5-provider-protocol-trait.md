# Extract per-provider `Protocol` trait with `step()` state machine

**Status**: done
**Milestone**: R5
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: r5-extract-tool-stream
**Blocks**: none

## Description

The OpenAI SSE parser in `crates/runie-provider/src/openai/stream.rs` is a monolithic async generator (`openai_event_stream` at line 59) that hard-codes HTTP transport, SSE framing, chunk parsing, and `LLMEvent` emission in one function. Adding a second provider (Anthropic, Ollama, Google) would duplicate the entire function with only the per-chunk parsing differing. OpenCode factors this into a `Protocol<Body, Frame, Event, State>` trait where `stream.step(state, event) -> (State, LLMEvent[])` is the only provider-specific logic; transport, framing, and accumulation are shared. Extract a Rust `ProviderProtocol` trait so new providers implement a ~40-line `step` function instead of a ~300-line stream generator.

## Acceptance Criteria

- [ ] New module `crates/runie-provider/src/protocol.rs` declares:
  - `pub trait ProviderProtocol: Send + Sync` with `fn initial(&self, request: &Request) -> Self::State;`, `fn step(&self, state: Self::State, frame: Self::Frame) -> (Self::State, Vec<LLMEvent>);`, `fn on_halt(&self, state: Self::State) -> Vec<LLMEvent>;` (default impl returns `vec![]`), and `fn terminal(&self, frame: &Self::Frame) -> bool;` (default impl returns `false`).
  - Associated types `type Frame;`, `type State;`.
- [ ] New module `crates/runie-provider/src/framing.rs` declares `pub fn sse_framing(bytes: impl Stream<Item = Bytes>) -> impl Stream<Item = String>` that splits a byte stream on `\n`, strips `data: ` prefixes, drops `[DONE]`, and yields JSON strings. (Extracted from `drain_buffer`/`parse_sse_event` in `openai/stream.rs`.)
- [ ] `crates/runie-provider/src/openai/stream.rs` is rewritten to: (1) build the HTTP request via `send_openai_request`, (2) pipe `response.bytes_stream()` through `sse_framing`, (3) parse each JSON string into an `OpenAiChunk` (`Frame`), (4) feed `(state, chunk)` into `OpenAiProtocol::step` which returns `(state, Vec<LLMEvent>)`, (5) yield those events. The `OpenAiProtocol` impl lives in a new `crates/runie-provider/src/openai/protocol.rs` (~120 LOC) and contains only the chunk-to-LLMEvent mapping (the current `process_chunk` + `process_tool_call_delta` + `flush_tool_calls` logic, refactored to use `ToolStream`).
- [ ] The public `openai_stream` function signature is unchanged — callers in `crates/runie-provider/src/openai/mod.rs` still get `Pin<Box<dyn Stream<Item = Result<LLMEvent>>>>`.
- [ ] `rg "fn openai_event_stream\b" crates/` returns zero hits (the generator is replaced by the protocol-driven loop).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds (all existing OpenAI stream tests pass).

## Tests

### Layer 1 — State/Logic
- [ ] `openai_protocol_step_text_delta` — `step(initial, chunk_with_content("hi"))` returns state with `Vec<TextDelta("hi")>` in the output.
- [ ] `openai_protocol_step_tool_call_accumulation` — feed three chunks (id+name, args part 1, args part 2 + finish_reason) through `step`; the third call returns `Vec[ToolCallEnd, Finish]` and the `ToolStream` is drained.
- [ ] `openai_protocol_step_reasoning_delta` — `step` on a chunk with `reasoning_content` returns `Vec[ThinkingDelta(...)]`.
- [ ] `openai_protocol_on_halt_flushes_pending` — start a tool call without a finish_reason chunk; `on_halt(state)` returns `Vec[ToolCallEnd]` for the pending tool.
- [ ] `openai_protocol_terminal_on_done` — `terminal(&OpenAiChunk::Done)` returns `true`.
- [ ] `sse_framing_splits_on_newline` — feed `b"data: {\"a\":1}\ndata: [DONE]\n"`; output is `["{\"a\":1}"]`.
- [ ] `sse_framing_handles_partial_chunks` — feed `b"data: {\"a\":"` then `b"1}\n"`; output is `["{\"a\":1}"]`.

### Layer 2 — Event Handling
- [ ] `openai_stream_emits_same_events_as_before` — existing stream tests in `openai/stream.rs` (lines 330-350) pass unchanged, proving the protocol-driven loop produces the same `LLMEvent` sequence as the monolithic generator.
- [ ] `openai_stream_handles_multi_tool_turn` — existing multi-tool fixture test passes.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_protocol_module_present` — `ls crates/runie-provider/src/protocol.rs crates/runie-provider/src/framing.rs` succeeds.
- [ ] `smoke_openai_protocol_module_present` — `ls crates/runie-provider/src/openai/protocol.rs` succeeds.

## Files touched

- `crates/runie-provider/src/protocol.rs` (new, ~50 LOC trait + associated types)
- `crates/runie-provider/src/framing.rs` (new, ~40 LOC SSE framing)
- `crates/runie-provider/src/openai/protocol.rs` (new, ~120 LOC `OpenAiProtocol` impl)
- `crates/runie-provider/src/openai/stream.rs` (rewrite to use `ProviderProtocol` loop, ~80 LOC down from 337)
- `crates/runie-provider/src/openai/mod.rs` (update imports)
- `crates/runie-provider/src/lib.rs` (add `pub mod protocol; pub mod framing;`)

## Notes

Source inspiration: OpenCode `packages/llm/src/route/protocol.ts` (84 LOC) and `packages/llm/src/route/client.ts:219-294` (`makeFromTransport`). The Rust version is simpler because we don't have Effect-TS's `mapAccumEffect` — we use a manual `while let Some(frame) = framed_stream.next().await` loop that calls `protocol.step(state, frame)`. The `Provider` trait in `runie-core/src/provider.rs` stays as the public-facing interface; `ProviderProtocol` is an internal helper for provider authors. Future Anthropic/Ollama providers implement `ProviderProtocol` and get transport+framing for free. Keep `OpenAiChunk` as the `Frame` type (already defined as `Chunk` in `stream.rs:26`) — just rename to `OpenAiFrame` for clarity.
