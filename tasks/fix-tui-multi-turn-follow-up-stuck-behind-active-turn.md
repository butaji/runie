# Fix TUI multi-turn follow-up stuck behind active turn

**Status**: done
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: fix-tui-turn-complete-leaves-working-status-and-queued, fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

After a first turn (`list files`) completes, submitting a follow-up (`show tasks`) is added to the queue but never runs. The status bar shows `Working... 0.0s (2 queued)`, indicating the first turn never released the queue.

## Live Evidence

```
  .gitignore
  .ralph/
  ...

  ⠋ Working... 0.0s (2 queued)                         ↑0 ↓87.3k -/s 0%/128k ⛀
```

The follow-up produced no response.

## Acceptance Criteria

- [ ] After a completed turn, a follow-up user message starts a new turn.
- [ ] The queued counter increments while waiting and decrements when the turn starts.
- [ ] Multiple follow-ups execute sequentially.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `list files` followed by `show tasks` produces a second response.

## Tests

### Layer 1 — State/Logic
- [ ] `follow_up_after_complete_starts_new_turn` — after `Done`, a queued user message triggers `handle_run_if_queued` exactly once.

### Layer 2 — Event Handling
- [ ] `submit_after_done_enqueues_and_runs` — simulate two submits; assert the second starts after the first emits `Done`.

### Layer 3 — Rendering
- [ ] `multi_turn_renders_two_responses` — `TestBackend` shows two distinct assistant outputs.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_list_files_then_show_tasks` — live tmux script runs both prompts and asserts two `Turn completed` lines or two response blocks.

## Files touched

- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/model/app_state.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is almost certainly the same root cause as the stuck `Working... (1 queued)` status after turn completion. It is split out to ensure multi-turn behavior is explicitly verified.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
