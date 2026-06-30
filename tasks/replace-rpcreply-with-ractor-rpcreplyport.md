# Replace RpcReply with ractor::RpcReplyPort

## Status

`todo`

## Context

`crates/runie-core/src/actors/ractor_adapter.rs:99-127` defines a custom `RpcReply<T>` as an `Arc<Mutex<Option<oneshot::Sender<T>>>>` plus a manual `rpc_channel()` helper. `ractor` already provides `RpcReplyPort` via `call!`/`call_t!`.

## Goal

Use `ractor::call!`/`call_t!` and `ractor::RpcReplyPort<T>`. Delete `RpcReply`, `rpc_channel`, and the `Reply` alias.

## Acceptance Criteria

- [ ] Remove `RpcReply`/`rpc_channel`/`Reply` from `ractor_adapter.rs`.
- [ ] Update all callers to use ractor macros.
- [ ] All RPC-style actor tests pass.

## Design Impact

No change to TUI element design or composition. Only internal actor RPC plumbing changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for actor request/reply.
- **Layer 2 — Event Handling:** Actor replies produce the same facts.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay fixture passes.
- **Live tmux validation:** TUI actor flow works.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
