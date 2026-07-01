# Compare quit, abort, and error recovery and fix gaps

**Status**: todo
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P0

**Depends on**: build-runie-vs-grok-build-comparison-harness, fix-tui-quit-shortcuts-ignored-during-active-turn
**Blocks**: none

## Description

Compare how Grok Build and Runie handle quitting, aborting an active turn, and recovering from errors. Grok Build supports `/quit`, Esc, and Ctrl+c. Runie's quit shortcuts are ignored during active turns. Fix gaps with unit + E2E tests.

## Scenario Set

1. Quit from idle state.
2. Quit during an active streaming turn.
3. Abort an active turn with a shortcut.
4. Encounter a provider error and observe recovery.
5. Invalid slash command error UX.

## Acceptance Criteria

- [ ] Each scenario runs in both tools.
- [ ] Runie `Ctrl+c`/`Ctrl+q` quit during active turn works.
- [ ] Runie `Ctrl+s` abort returns to idle.
- [ ] Provider/offline errors produce clear messages and return to idle.
- [ ] Actionable findings become tasks with unit + E2E + live tmux AC.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 2 — Event Handling
- [ ] `ctrl_c_quits_during_turn` — `Ctrl+c` emits `Quit` even when `turn_active`.
- [ ] `ctrl_s_aborts_during_turn` — `Ctrl+s` emits `Abort` and cancels the turn.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_quit_during_hello_loop` — live tmux script starts `hello` and quits successfully.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-tui/src/keymap.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`

## Fixture / Replay Strategy

Use recorded Grok Build TUI pane fixtures for quit/abort/error scenarios. Derive Runie `TestBackend` expected buffers from the pane dumps. Do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Overlaps with `fix-tui-quit-shortcuts-ignored-during-active-turn`.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
