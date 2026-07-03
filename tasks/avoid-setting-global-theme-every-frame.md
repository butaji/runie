# Avoid setting global theme every frame

**Status**: done
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P3

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

`render_loop` calls `theme::set_current_theme_with_caps(&snap.theme_name, caps)` on every frame (~16 ms). This relies on a global mutable theme store and adds minor overhead and impurity to the render path.

## Root Cause

`crates/runie-tui/src/main.rs:257` sets the theme unconditionally inside the render loop.

## Acceptance Criteria

- [ ] The theme is set only when the theme name or terminal capabilities change.
- [ ] The render path remains a pure function of the snapshot.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux theme switching still works.

## Tests

### Layer 3 — Rendering
- [ ] `theme_set_only_on_change` — snapshot with the same theme does not call the global setter.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — covered by existing render tests.

## Files touched

- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/theme.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Minor polish; lower priority than functional bugs.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
