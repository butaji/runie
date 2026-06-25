# Generic actor reply wrapper

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: reduce-actor-handle-boilerplate

## Description

`runie-core/src/actors/config/messages.rs` defines `ConfigReply<T>` and `runie-core/src/actors/provider/messages.rs` defines `ProviderReply<T>`. Both were `Arc<Mutex<Option<oneshot::Sender<T>>>>` with identical `new`, `send`, and `Clone` implementations.

## Acceptance Criteria

- [x] A generic `runie_core::actor::Reply<T>` replaces both wrappers.
- [x] `ConfigReply` and `ProviderReply` are deleted.
- [x] All actor message modules use the generic reply.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `reply_send_delivers_value` — generic reply delivers a value to the receiver.
- [x] `reply_clone_shares_sender` — cloned reply sends to the same channel.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `config_actor_uses_generic_reply` — config actor round-trip still works.
- [x] `provider_actor_uses_generic_reply` — provider actor round-trip still works.

## Files touched

- `crates/runie-core/src/actors/trait.rs` — defines generic `Reply<T>`
- `crates/runie-core/src/actors/config/messages.rs` — uses `Reply<T>`
- `crates/runie-core/src/actors/provider/messages.rs` — uses `Reply<T>`

## Notes

Generic `Reply<T>` is defined in `actors/trait.rs` and both config and provider actor message modules already use it. `ConfigReply` and `ProviderReply` wrappers no longer exist.
