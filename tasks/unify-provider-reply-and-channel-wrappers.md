# Unify provider reply and channel wrappers

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

`crates/runie-core/src/actors/provider/messages.rs` defines a private `Reply<T>` that duplicates `ractor_adapter::Reply`/`RpcReply`. `ProviderActorHandle` duplicates `RactorProviderHandle`. Delete the duplicates and use the shared wrapper.

## Acceptance Criteria

- [ ] Delete `Reply<T>` from `provider/messages.rs`; import from `ractor_adapter`.
- [ ] Delete dead `ProviderActorHandle`; use `RactorProviderHandle` or bare `ActorRef<ProviderMsg>`.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `provider_reply_wrapper_is_shared` — only one `Reply` type remains.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `provider_replay_uses_shared_reply` — turn completes.

## Files touched

- `crates/runie-core/src/actors/provider/messages.rs`
- `crates/runie-core/src/actors/provider/ractor_provider.rs`
- `crates/runie-core/src/actors/provider/mod.rs`
- `crates/runie-core/src/actors/ractor_adapter.rs`

## Notes

- Keep behavior identical; this is a code-size simplification.
