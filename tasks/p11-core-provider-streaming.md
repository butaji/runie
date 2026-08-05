# p11 — Core provider: streaming partial reconstruction, error/aborted encoding, abort wiring

**Parity target:** pi proxy stream function + stream shape.

## Pi reference

`~/Code/agents/pi/packages/agent/src/proxy.ts`
- `streamProxy` (line 116) POSTs `{model, context, options}` and parses SSE `data: <json>` lines.
- `processProxyEvent` (line 238) reconstructs the partial **client-side**:
  - partial init: `{role:"assistant", stopReason:"pending", content:[], ...usage:zero-filled}` (line 121).
  - `text_start` sets `content[idx]={type:"text",text:""}`; `text_delta` appends; `text_end` sets `textSignature`.
  - `thinking_start`/`delta`/`end` analogous.
  - `toolcall_start` sets `{type:"toolCall", id, name, arguments:{}, partialJson:""}` (line 310); `toolcall_delta` appends `partialJson` and `arguments = parseStreamingJson(partialJson) || {}`; `toolcall_end` deletes `partialJson` and yields the toolCall.
  - `done` sets `stopReason`, `usage` → `{type:"done", reason, message: partial}`.
  - `error` sets `stopReason`, `errorMessage`, `usage` → `{type:"error", reason, error: partial}`.
  - On fetch failure/abort: reason `aborted` if signal aborted else `error` (line 214-224).
- Default stream fn — `~/Code/agents/pi/packages/agent/src/stream-fn.ts:15`: throws `"No default stream function configured..."` if unset.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/provider/`
- `replay.rs` (SSE from file/http), `stream_fn.rs` (StreamFn trait + AssistantMessageEventStream), `actor.rs`, `http.rs`.
- `AssistantMessageEvent` (types.rs:413) is coarse (see p04); the provider emits events directly rather than reconstructing a partial.

## Adapt to runie

1. Match pi's failure **encoding contract**: the provider must encode failures as an assistant stream ending with `Error{error}` (or `Done{stop_reason: Error|Aborted}`) rather than a bare channel drop, so the loop's `apply_event` sets `stop_reason` correctly (driver.rs).
2. Abort wiring: `SimpleStreamOptions.signal` (types.rs:300) must propagate to the provider and into a `CancellationToken` for tool execution; aborted mid-stream yields `Error{reason: aborted}`.
3. Streaming JSON target for `toolcall_delta`: implement `parse_streaming_json` (salvage parser) so `ToolCallDelta.partial.arguments` accumulates from partial JSON (feeds p04).
4. Multi-turn replay: the replay provider must support **per-turn sequences** so p05 auto-continue can be tested (second call returns a terminating Stop ack). Extend `replay.rs`/`ScenarioStream` with a turn index.
5. `set_default_stream_fn`/`get_default_stream_fn` singleton (Rust: `OnceLock<Arc<dyn StreamFn>>`), throws the pi message when unset and no explicit stream passed.

## State machine / variants

Stream lifecycle (per content index, per pi):
```
idle → start → text_start→text_delta*→text_end | thinking_start→thinking_delta*→thinking_end | toolcall_start→toolcall_delta*→toolcall_end
     → done(stop|length|toolUse) | error(aborted|error) | transport-fail(aborted|error)
```
Replay turn machine: `turn[0] → turn[1] → ... → Stop` (each `stream()` call advances the turn index).

## Acceptance

- Provider tests: SSE with `done{stop}`/`error{aborted}`/truncated tool JSON; `parse_streaming_json` salvage; abort mid-stream yields `Error{aborted}`; multi-turn replay advances per call.
- `cargo test -p runie-core` green.