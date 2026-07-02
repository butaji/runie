# Replace RpcReply with ractor::RpcReplyPort

## Status

**done** ✅

## Context

`crates/runie-core/src/actors/ractor_adapter.rs:99-127` defined a custom `RpcReply<T>` as an `Arc<Mutex<Option<oneshot::Sender<T>>>>` plus a manual `rpc_channel()` helper. `ractor` already provides `RpcReplyPort` via `call!`/`call_t!`.

## Goal

Use `ractor::call!`/`call_t!` and `ractor::RpcReplyPort<T>`. Delete `RpcReply`, `rpc_channel`, and the `Reply` alias.

## Changes Made

The task was already completed in previous iterations:

1. The custom `RpcReply` type was removed from `ractor_adapter.rs`.
2. The codebase now uses `ractor::RpcReplyPort<T>` directly in actor message definitions.
3. All actor message handlers use the standard ractor pattern with `RpcReplyPort`.

### Current Usage

All actors now use `ractor::RpcReplyPort` in their message definitions:

- `config/messages.rs`: `GetConfig(RpcReplyPort<Config>)`, `GetConfiguredProviders`, etc.
- `provider/messages.rs`: `GetBuiltProvider(RpcReplyPort<Result<BuiltProvider, ProviderError>>)`
- `turn/messages.rs`: `DeliverQueued(RpcReplyPort<Option<DeliverQueuedResponse>>)`
- `permission/messages.rs`: Uses `RpcReplyPort` for permission responses

## Acceptance Criteria

- [x] Remove `RpcReply`/`rpc_channel`/`Reply` from `ractor_adapter.rs`.
- [x] Update all callers to use ractor macros.
- [x] All RPC-style actor tests pass.

## Tests

### Evidence

- All tests pass: `cargo test --workspace`
- `ractor_adapter.rs` no longer defines custom RPC types
- All actor messages use `ractor::RpcReplyPort`
