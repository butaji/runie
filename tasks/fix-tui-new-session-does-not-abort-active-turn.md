# Fix TUI /new session does not abort an active turn

**Status**: todo
**Milestone**: R7
**Category**: Sessions
**Priority**: P1

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

Running `/new` while a turn is still streaming starts a new session in the UI but leaves the previous agent turn running. In mock mode the repeated `hello` output continues to appear below the "New session started" message, and the status remains `Working...`.

## Live Evidence

```
  New session started

  →  ◐ 0.0s

 hello hello hello hello hello hello hello hello hello hello hello hello hello
 ...

  ⠦ Working... 6.5s (1 queued)                        ↑0 ↓147.4k -/s 0%/128k ⛀
```

## Acceptance Criteria

- [ ] `/new` aborts any in-flight provider stream and tool calls.
- [ ] The previous turn stops emitting events before the new session UI is rendered.
- [ ] The status bar returns to idle after `/new`.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `/new` during a mock `hello` turn stops the repetition.

## Tests

### Layer 1 — State/Logic
- [ ] `new_session_aborts_active_turn` — after starting a turn, `handle_new` sets `turn_active == false` and clears the queue.

### Layer 2 — Event Handling
- [ ] `new_command_emits_abort` — `/new` event emits `TurnMsg::AbortTurn` and `Event::NewSession` in the correct order.

### Layer 3 — Rendering
- [ ] `new_session_renders_idle` — `TestBackend` shows `New session started` and an idle input status.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_new_aborts_hello_loop` — live tmux script starts `hello`, waits 3s, runs `/new`, and asserts the pane stops accumulating `hello` tokens.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-tui/src/ui_actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is a safety/correctness issue: an infinite mock turn could leak across sessions.
- The abort path should cancel the provider stream future, not just clear UI state.
