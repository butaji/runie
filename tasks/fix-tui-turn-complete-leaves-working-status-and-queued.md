# Fix TUI turn-complete leaves Working status and queued request

**Status**: todo
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: fix-tui-multi-turn-follow-up-stuck-behind-active-turn

## Description

After a tool turn completes (e.g. `list files` in mock mode), the assistant area correctly shows `Turn completed in 0.0s`, but the status bar still reads `Working... 0.0s (1 queued)` and the input hint still shows steering/follow-up keys. The queued request is not cleared, so the TUI never returns to the idle state.

## Live Evidence

```
  config.schema.json
  crates/
  ...

  →  ◐ 0.1s

  Turn completed in 0.0s

  ⠧ Working... 0.0s (1 queued)                         ↑0 ↓61.2k -/s 0%/128k ⛀
```

## Acceptance Criteria

- [ ] After a turn reaches `TurnComplete`/`Done`, the status bar leaves `Working...` and shows the idle prompt.
- [ ] The queued-request counter drops to zero.
- [ ] The input hint returns to the idle set (no `enter steer` / `alt+enter follow-up`).
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `list files` scenario shows an idle status after `Turn completed`.

## Tests

### Layer 1 — State/Logic
- [ ] `turn_complete_clears_queue_and_status` — after a completed turn, assert `turn_active == false` and the request queue is empty.

### Layer 2 — Event Handling
- [ ] `done_event_updates_status_to_idle` — feed `Event::Done`/`Event::TurnComplete` and assert the idle state events are emitted.

### Layer 3 — Rendering
- [ ] `completed_turn_renders_idle_status` — `TestBackend` asserts the status line no longer contains `Working` after completion.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_list_files_reaches_idle` — live tmux script checks the captured pane for `Turn completed` followed by an idle status.

## Files touched

- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-tui/src/state.rs` (if status lives in AppState)

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This may share a root cause with the `hello` repetition: if `TurnActor` does not transition out of `turn_active`, it may keep re-running the queue.
