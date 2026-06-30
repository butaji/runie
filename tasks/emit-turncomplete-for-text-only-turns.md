# Emit TurnComplete for text-only turns

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: fix-tui-turn-complete-leaves-working-status-and-queued
**Blocks**: fix-tui-mock-simple-text-response-repetition

## Description

`TurnComplete` is currently emitted only when `has_intermediate_steps` is true (i.e. the turn involved tool calls). A plain text response emits only `Response`/`Done`, so UI components that key on `TurnComplete` for timing, sound, or status transitions miss non-tool turns.

## Root Cause

The emission logic in `crates/runie-agent/src/turn/mod.rs` gates `TurnComplete` on the presence of intermediate steps.

## Acceptance Criteria

- [ ] `TurnComplete` is emitted unconditionally at the end of every turn.
- [ ] `has_intermediate_steps` is used only for tool-specific side effects (e.g. showing tool summaries), not for deciding whether to emit `TurnComplete`.
- [ ] The TUI status returns to idle for both text-only and tool turns.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `hello` scenario reaches idle.

## Tests

### Layer 1 — State/Logic
- [ ] `text_turn_emits_turn_complete` — a mock text response produces `TurnComplete`.
- [ ] `tool_turn_emits_turn_complete` — a mock tool turn also produces `TurnComplete`.

### Layer 2 — Event Handling
- [ ] `ui_actor_turn_complete_clears_status` — `Event::TurnComplete` transitions the status to idle.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_hello_reaches_idle` — live tmux script asserts idle status after `hello`.

## Files touched

- `crates/runie-agent/src/turn/mod.rs`
- `crates/runie-core/src/event.rs`
- `crates/runie-tui/src/ui_actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This may be the missing signal that prevents the TUI from leaving `Working...` after the mock `hello` echo.
