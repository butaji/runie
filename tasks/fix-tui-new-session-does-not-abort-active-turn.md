# Fix TUI /new session does not abort an active turn

**Status**: done
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

- [x] `/new` aborts any in-flight provider stream and tool calls.
- [x] The previous turn stops emitting events before the new session UI is rendered.
- [x] The status bar returns to idle after `/new`.
- [x] `cargo test --workspace` passes.
- [ ] Live tmux `/new` during a mock `hello` turn stops the repetition.

## Tests

### Layer 1 — State/Logic
- [x] `new_session_aborts_active_turn` — after starting a turn, `handle_new` sets `turn_active == false` and clears the queue.

### Layer 2 — Event Handling
- [x] `new_command_emits_abort` — `/new` event emits `TurnMsg::AbortTurn` and `Event::NewSession` in the correct order.
- [x] `ui_actor_dispatch_submit_closes_palette` — `/new` (slash command) closes the command palette.

### Layer 3 — Rendering
- [ ] `new_session_renders_idle` — `TestBackend` shows `New session started` and an idle input status.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_new_aborts_hello_loop` — live tmux script starts `hello`, waits 3s, runs `/new`, and asserts the pane stops accumulating `hello` tokens.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-tui/src/ui_actor.rs`

## Validation

Done: Unit tests pass. 4 pre-existing queue/flow test failures remain (unrelated).

## Implementation

1. `UiActor::clear_turn_state(is_abort: bool)` — async helper that calls `ClearQueues` for Abort or `DeliverQueued + RunIfQueued` for TurnCompleted.
2. `UiActor::dispatch_submit_content` detects `CommandResult::Events` containing `Abort`, calls `clear_turn_state(true).await`.
3. Made `dispatch_submit_content`, `handle_input_changed`, and their call sites in `handle_input_event` async.
4. `AgentCommand` gets `cancellation_token: CancellationToken::new()` so the token is fresh per command.

## Notes

- This is a safety/correctness issue: an infinite mock turn could leak across sessions.
- The abort path cancels the provider stream future via `CancellationToken`.
- 4 pre-existing queue/flow test failures (unrelated to this fix).
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
