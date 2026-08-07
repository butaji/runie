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

## Progress

- **Cancellation ownership (2026-08-05):** `ProviderCommand::Cancel` now
  aborts all active stream pumps through the provider actor's owned `JoinSet`;
  a pending-stream regression test verifies cancellation closes the receiver
  without leaving a detached task.
- **Cancellation acknowledgement (2026-08-06):** `ProviderActor::cancel`
  now waits for the actor worker to process `ProviderCommand::Cancel` and
  abort its owned pumps before returning. This makes the cancellation boundary
  deterministic for the loop and replay callers.
- **Startup error encoding (2026-08-05):** provider startup failures now
  deliver an `AssistantMessageEvent::Error` through the subscribed stream,
  preserving pi's error-event lifecycle instead of presenting an empty stream.
  A provider actor regression test pins the exact error payload.
- **Abort signal propagation (2026-08-05):** the loop now passes its abort
  watch through `SimpleStreamOptions.signal` to every provider stream call;
  integration coverage confirms adapters receive the signal.

## Acceptance

- Provider tests: SSE with `done{stop}`/`error{aborted}`/truncated tool JSON; `parse_streaming_json` salvage; abort mid-stream yields `Error{aborted}`; multi-turn replay advances per call.
- `cargo test -p runie-core` green.
## Progress

- **Provider hook options (2026-08-05):** `SimpleStreamOptions` now carries
  pi-compatible async `on_payload` and `on_response` hooks through the loop
  boundary. Provider adapters receive them unchanged; a two-turn integration
  test verifies both hook fields survive each request.
- **Transport hook execution (2026-08-05):** `HttpActor::post_with_options`
  now applies payload transformations before the actor-owned request and
  delivers response status/headers to the response hook. Replay transport has
  focused coverage for both callbacks and exposes the path through
  `ReplayProvider::from_http_with_options`.
