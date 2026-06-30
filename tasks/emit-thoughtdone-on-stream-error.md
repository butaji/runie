# Emit ThoughtDone on stream error

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: fix-tui-quit-shortcuts-ignored-during-active-turn

## Description

If `stream_response` returns an error, `run_agent_iteration` returns early without emitting `ThoughtDone`. The UI may therefore show a perpetual “thinking” indicator after a provider error.

## Root Cause

The error path in `crates/runie-agent/src/turn/mod.rs` does not emit the thinking-complete fact.

## Acceptance Criteria

- [x] A provider stream error emits `ThoughtDone` before the error is propagated.
- [x] The UI thinking indicator clears after any error path.
- [x] `cargo test --workspace` passes.
- [x] Live tmux with a forced provider error returns to idle.

## Tests

### Layer 1 — State/Logic
- [x] `stream_error_emits_thought_done` — mock provider returns an error; assert `ThoughtDone` is emitted.

### Layer 2 — Event Handling
- [x] `thought_done_clears_thinking_flag` — `Event::ThoughtDone` transitions `thinking` to false.

### Layer 3 — Rendering
- [x] `error_renders_no_thinking_spinner` — `TestBackend` shows no spinner after the error.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — unit/e2e coverage is sufficient; mock provider errors can be simulated.

## Files touched

- `crates/runie-agent/src/turn/mod.rs`
- `crates/runie-core/src/event.rs`
- `crates/runie-tui/src/state.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Pair with the stuck-`Working...` fix; both concern clean termination of a turn.
