# Step 08: ProviderActor + StreamFn trait

**Status:** implemented; provider-specific deferred/WebSocket adapters remain open (2026-08-07)
**Depends on:** 07

## Goal
Implement the actor that owns the one in-flight LLM stream and the abstract `StreamFn` trait that backs it.

## Changes
- `crates/runie-core/src/provider/stream_fn.rs`:
  - `StreamFn` trait (async, send + sync): `async fn stream(&self, model, context, options) -> AssistantMessageEventStream`.
  - `AssistantMessageEventStream = Pin<Box<dyn Stream<Item = AssistantMessageEvent> + Send>>`.
  - `SimpleStreamOptions` struct mirroring TS: `session_id`, `thinking_budgets`, `api_key`, `signal`.
  - `StreamError` enum (Network, Api, Aborted, Invalid).
  - `Model` struct (id, name, api, provider, base_url, reasoning, input, cost, context_window, max_tokens).
- `crates/runie-core/src/provider/actor.rs`:
  - `ProviderActor` with `mpsc::Sender<ProviderCommand>`.
  - `ProviderCommand::Start { model, context, options, reply: oneshot::Sender<broadcast::Receiver<AssistantMessageEvent>> }`, `Cancel`, `WaitForIdle`.
  - Worker holds the `Arc<dyn StreamFn>`, current cancellation token, and `broadcast::Sender<AssistantMessageEvent>` (capacity `EVENT_CAPACITY = 1024`).
- `crates/runie-core/src/provider/mod.rs`: re-exports.

## Verification
- `cargo check -p runie-core` → exit 0.
- Unit test: `MockStreamFn` (defined in step 11) emits 3 events; actor forwards all 3 to subscribers.

## Current parity boundary

`ProviderActor` now owns the in-flight stream pump, cancellation, replacement
of superseded runs, and `FetchDeferred`/`CancelDeferred` mailbox commands.
Unsupported adapter capabilities return explicit `StreamError` values rather
than being silently reinterpreted as HTTP or SSE. YAML replay covers the
deferred event-to-state contract; provider-specific polling and WebSocket wire
adapters remain separate implementation slices.

## Notes
- The actor is the **only** site that owns the cancellation token. The loop actor requests cancellation via `actor.cancel()`.
- Stream is held for the duration of one assistant turn; the actor rejects a second `Start` until the first completes (or returns an error).
