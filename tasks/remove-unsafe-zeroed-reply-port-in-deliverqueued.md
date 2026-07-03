# Remove unsafe zeroed reply port in `DeliverQueued`

## Status

`done`

## Description

`TurnMsg::DeliverQueued` no longer uses `unsafe { std::mem::zeroed() }` for the `RpcReplyPort`. The message uses `reply: None` for fire-and-forget calls.

## Implementation

Commit `f54b71eb fix: eliminate all unsafe mem::zeroed() reply-port patterns` removed all unsafe zeroed reply port patterns.

The `Clone for TurnMsg` implementation sets `reply: None` for `DeliverQueued`, making it safe to clone without risking use-after-free on the reply port.

## Acceptance criteria

1. **Unit tests** ✅ — No `mem::zeroed` in codebase; all tests pass.
2. **E2E tests** ✅ — Queue delivery works correctly in replay tests.
3. **Live tmux tests** ✅ — Queue delivery works in tmux.

## Tests

All tests pass, including queue-related tests.
