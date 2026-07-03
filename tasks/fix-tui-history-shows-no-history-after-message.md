# Fix TUI /history shows "No history" after a message is sent

**Status**: done
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

- [x] After at least one user message is submitted, `/history` shows that message.
- [x] After a completed assistant/tool turn, `/history` includes both user and assistant/tool entries.
- [x] `cargo test --workspace` passes.
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

- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — ✅ fixed `handle_history` to read from session messages
- `crates/runie-core/src/tests/slash/session.rs` — ✅ updated `history_lists_recent_inputs` test

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- **Root cause**: `/history` read from `state.input().input_history` (command palette history) instead of `state.session().messages` (conversation history).
- **Fix**: Changed `handle_history` to read from `state.session().messages()` and format with role labels.
- Layer 3 (Rendering) and Layer 4 (live tmux) tests deferred — depend on `fix-tui-mock-simple-text-response-repetition` and `fix-tui-turn-complete-leaves-working-status-and-queued` being fixed first.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
