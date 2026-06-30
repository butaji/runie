# Unify provider reply and channel wrappers

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

`crates/runie-core/src/actors/provider/messages.rs` defined a private `Reply<T>` that duplicated `ractor_adapter::Reply`/`RpcReply`. `ProviderActorHandle` duplicated `RactorProviderHandle`. Both duplicates have been deleted; all provider message handling now uses the shared `rpc_channel()` and `RpcReply::send()` from `ractor_adapter`.

## Acceptance Criteria

- [x] Delete `Reply<T>` from `provider/messages.rs`; import from `ractor_adapter`.
- [x] Delete dead `ProviderActorHandle`; use `RactorProviderHandle` or bare `ActorRef<ProviderMsg>`.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `provider_reply_wrapper_is_shared` — only one `Reply` type remains (verified by compilation).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `provider_replay_uses_shared_reply` — all provider tests pass.

## Files touched

- `crates/runie-core/src/actors/provider/messages.rs` — removed local `Reply<T>`, `make_reply`, `take_reply`, `ProviderActorHandle`; `ProviderMsg` now uses `Reply` from `ractor_adapter`
- `crates/runie-core/src/actors/provider/ractor_provider.rs` — uses `rpc_channel()` from `ractor_adapter`; removed `From<RactorProviderHandle>` impl
- `crates/runie-core/src/actors/provider/mod.rs` — removed `ProviderActorHandle` re-export
- `crates/runie-core/src/actors/mod.rs` — removed `ProviderActorHandle` re-export

## Notes

- `ractor_adapter::Reply` is an alias for `RpcReply<T>`; `rpc_channel()` is the canonical way to create reply channels.
- `ProviderActorHandle` was a dead re-export; `RactorProviderHandle` is the correct type.
- The `RpcReply::send(self, value)` API replaces the old `take_reply(&reply)` + `tx.send(result)` pattern.
