# Offload provider network calls from actor handler

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: propagate-actor-spawn-errors-instead-of-panicking
**Blocks**: live-tui-smoke-test-real-minimax

## Description

`RactorProviderActor` awaits HTTP/credential calls directly inside `handle` for `ValidateKey` and `ListModels`. This blocks the provider actor’s mailbox, so other provider requests queue behind a slow network call.

## Root Cause

The actor message handler performs network IO inline instead of spawning a task and replying asynchronously.

## Acceptance Criteria

- [ ] Network calls for `ValidateKey` and `ListModels` run outside the actor’s `handle` method.
- [ ] Results are sent back to the actor via its normal message channel or RPC reply.
- [ ] The actor mailbox remains responsive during validation/listing.
- [ ] `cargo test --workspace` passes.
- [ ] Live MiniMax model listing does not block other provider requests.

## Tests

### Layer 1 — State/Logic
- [ ] `provider_actor_mailbox_not_blocked_by_validate` — `ListModels` can be processed while `ValidateKey` is in flight.

### Layer 2 — Event Handling
- [ ] `validate_key_result_event` — the async task result produces the expected fact event.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A for mock; real MiniMax test validates responsiveness.

## Files touched

- `crates/runie-core/src/actors/provider/ractor_provider.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This matters most for real providers where network latency is significant.
