# Remove `Clone` impl for messages with reply ports

## Status

`todo`

## Description

`TurnMsg::DeliverQueued` derives `Clone`, which zeros the reply port. Remove `Clone` for messages containing `RpcReplyPort`.

## Acceptance criteria

1. **Unit tests** — Messages with reply ports cannot be cloned; compilation catches misuse.
2. **E2E tests** — Replay still passes.
3. **Live tmux tests** — Not applicable.

## Tests

### Unit tests
- Compilation fails if clone attempted.

### E2E tests
- Existing replay.

### Live tmux tests
- N/A.
