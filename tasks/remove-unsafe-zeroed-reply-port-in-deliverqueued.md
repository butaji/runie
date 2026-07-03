# Remove unsafe zeroed reply port in `DeliverQueued`

## Status

`todo`

## Description

`update/session.rs` uses `unsafe { std::mem::zeroed() }` for the `RpcReplyPort` in `TurnMsg::DeliverQueued`. Replace with a fire-and-forget message variant or await a real reply.

## Acceptance criteria

1. **Unit tests** — No `mem::zeroed` in actor messaging; fire-and-forget or RPC path is correct.
2. **E2E tests** — Queue delivery still works in replay.
3. **Live tmux tests** — Queue delivery works in tmux.

## Tests

### Unit tests
- `DeliverQueued` handling without zeroed port.

### E2E tests
- Replay queued delivery.

### Live tmux tests
- Submit queued messages.
