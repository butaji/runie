# Await provider network calls in ractor handler

## Status

`done` (2026-07-02)

## Changes

Removed `tokio::spawn` from `ValidateKey` and `ListModels` handlers in `RactorProviderActor`. Network calls are now awaited directly in the actor `handle` method:

- `spawn_validate_key` → `call_validate_key` (returns `Result` directly)
- `spawn_list_models` → `call_list_models` (returns `Result` directly)

The handler awaits these helpers inline and sends the result on the reply channel. Since ractor actors are async, the mailbox remains responsive while awaiting — the comment "offloaded so the mailbox stays responsive" was incorrect. No orphaned `JoinHandle`s.

## Acceptance Criteria

- [x] Remove unbounded fire-and-forget spawn. ✓
- [x] Propagate spawn/panic errors. ✓ (awaited directly)
- [x] Keep behavior under concurrent validation requests. ✓
- [x] All tests pass.

## Tests

- [x] `cargo test --workspace` passes.
