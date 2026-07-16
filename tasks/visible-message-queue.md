# Visible message queue indicator

## Objective

Show the number of queued follow-up messages in the status bar so users know a follow-up is waiting to be sent.

## Agent landscape finding

codex queues drafts with `Tab`; gemini and kimi show queued-message indicators. Runie has `Alt+Enter` for follow-ups but no visibility.

## runie current state

Runie supports `Alt+Enter` to queue a follow-up during an active turn, but the queued message is not visible until the turn completes.

## Required runie changes

- Display a queued-message count in the status bar (e.g., `· 1 queued`).
- Update the count when follow-ups are added or dequeued.
- Keep existing `Alt+Enter` behavior unchanged.

## Test scenarios

1. **Queue count appears after Alt+Enter**
   - Keys: trigger a long response, press `M-Enter`, type `follow up`, press `Enter`
   - Assert: status bar shows queued count `1` while turn is active.

2. **Count clears after delivery**
   - Keys: wait for turn to complete.
   - Assert: queued count disappears; follow-up is sent.

3. **Multiple queues increment**
   - Keys: queue two follow-ups.
   - Assert: status bar shows `2 queued`.

## Edge / negative cases

- Dequeuing a message (`Alt+Up`) decrements the count.
- Count is hidden when zero.

## Dependencies

- `turn_lifecycle`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()`.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
