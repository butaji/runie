# Fix TUI mock simple-text response repetition

**Status**: todo
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: implement-graceful-leader-shutdown
**Blocks**: live-tui-smoke-test-real-minimax

## Description

During live `tmux` smoke tests with the mock provider, typing "hello" causes the assistant response area to fill with repeated "hello" tokens and the turn stays in "Working..." indefinitely. The expected behavior is a single echo response followed by `TurnComplete`.

## Acceptance Criteria

- [ ] Typing a simple prompt (e.g. "hello") in mock mode renders a short, non-repeating response.
- [ ] The turn completes and the UI returns to the idle input state.
- [ ] `cargo test --workspace` passes.
- [ ] `scripts/tmux-smoke-test.sh mock` passes.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_hello_turn_completes` — tmux smoke test for "hello" reaches idle state within 10s.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-tui/src/message/mod.rs`
- `crates/runie-agent/src/turn/mod.rs`
- `crates/runie-provider/src/mock.rs`

## Notes

- Could be a mock-provider echo loop, a `PacedRenderer` accumulation bug, or missing `TurnComplete` handling.
