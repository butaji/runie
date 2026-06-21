# Generic actor reply wrapper

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: reduce-actor-handle-boilerplate

## Description

`runie-core/src/actors/config/messages.rs` defines `ConfigReply<T>` and `runie-core/src/actors/provider/messages.rs` defines `ProviderReply<T>`. Both are `Arc<Mutex<Option<oneshot::Sender<T>>>>` with identical `new`, `send`, and `Clone` implementations. The comment in `ProviderReply` incorrectly claims it differs from `ConfigReply`.

## Acceptance Criteria

- [ ] A generic `runie_core::actor::Reply<T>` replaces both wrappers.
- [ ] `ConfigReply` and `ProviderReply` are deleted.
- [ ] All actor message modules use the generic reply.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `reply_send_delivers_value` — generic reply delivers a value to the receiver.
- [ ] `reply_clone_shares_sender` — cloned reply sends to the same channel.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `config_actor_uses_generic_reply` — config actor round-trip still works.
- [ ] `provider_actor_uses_generic_reply` — provider actor round-trip still works.

## Files touched

- `crates/runie-core/src/actors/config/messages.rs`
- `crates/runie-core/src/actors/provider/messages.rs`
- New `crates/runie-core/src/actors/reply.rs` or `crates/runie-core/src/actor.rs`

## Notes

The generic reply works for non-`Clone` `T` because the `Arc<Mutex<Option<...>>>` holds the sender, not the value.
