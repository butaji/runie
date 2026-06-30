# Fix TUI /history shows "No history" after a message is sent

**Status**: todo
**Milestone**: R7
**Category**: Sessions
**Priority**: P2

**Depends on**: fix-tui-mock-simple-text-response-repetition, fix-tui-turn-complete-leaves-working-status-and-queued
**Blocks**: none

## Description

After sending a user message in the TUI, `/history` reports `No history.` even though the message appears in the chat area. History should include the submitted user message (and any completed assistant/tool messages).

## Live Evidence

```
  ❯ hello

  ...repeating hello response...

  /history

  No history.
```

## Acceptance Criteria

- [ ] After at least one user message is submitted, `/history` shows that message.
- [ ] After a completed assistant/tool turn, `/history` includes both user and assistant/tool entries.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `hello` followed by `/history` shows the `hello` message.

## Tests

### Layer 1 — State/Logic
- [ ] `history_includes_submitted_user_message` — after `submit_user_message`, `handle_history` returns non-empty text containing the message.

### Layer 2 — Event Handling
- [ ] `history_command_after_submit_returns_history` — simulate `Submit` then `/history`, assert the resulting message event contains the input.

### Layer 3 — Rendering
- [ ] `history_result_renders_messages` — `TestBackend` shows the history message after `/history`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_history_after_hello` — live tmux script sends `hello`, waits for completion, runs `/history`, and asserts `hello` is in the pane.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- `crates/runie-core/src/model/app_state.rs`
- `crates/runie-core/src/actors/session/ractor_session.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This may be caused by the session state not being updated from `TurnActor` events, or by `/history` reading from a different source than the chat log.
- Fix after the hello repetition / turn-completion bugs, since those prevent a clean completed turn.
